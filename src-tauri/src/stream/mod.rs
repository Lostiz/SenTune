use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::api;
use crate::cache;
use crate::db;
use crate::models::{ApiError, AudioStream, VideoDetail};

pub mod server;

pub const STATUS_DOWNLOADING: u8 = 0;
pub const STATUS_COMPLETED: u8 = 1;
pub const STATUS_CANCELLED: u8 = 2;
pub const STATUS_FAILED: u8 = 3;

/// 下载分块大小（每次向 CDN 请求的字节数）。
const CHUNK_SIZE: u64 = 512 * 1024;
/// seek 跳转阈值：目标位置比当前游标超前超过该值才跳转。
const JUMP_THRESHOLD: u64 = 256 * 1024;
/// 读取端等待单块数据的默认超时。
pub const READ_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_WRITE: u32 = 0x2;
const FILE_SHARE_DELETE: u32 = 0x4;

static MANAGER: OnceLock<Arc<Mutex<HashMap<String, Arc<StreamTask>>>>> = OnceLock::new();

fn manager() -> &'static Arc<Mutex<HashMap<String, Arc<StreamTask>>>> {
    MANAGER.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

#[derive(Debug, Clone, Default)]
pub struct DownloadState {
    /// 已写入的字节区间（有序、不重叠、闭区间）。
    pub filled: Vec<(u64, u64)>,
}

pub struct StreamTask {
    pub id: String,
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub audio_urls: Vec<String>,
    pub url_index: AtomicUsize,
    pub audio_id: u64,
    #[allow(dead_code)]
    pub codec: String,
    pub content_type: Mutex<Option<String>>,
    pub total_size: AtomicU64,
    pub part_path: PathBuf,
    pub cache_path: PathBuf,
    pub status: AtomicU8,
    pub error: Mutex<Option<String>>,
    pub cancelled: Arc<AtomicBool>,
    /// 已连续填充的前缀长度（供“已下载区间”快速判断）。
    pub downloaded: AtomicU64,
    /// 读取端 seek 跳转提示；u64::MAX 表示无提示。
    pub jump_target: AtomicU64,
    pub state: Mutex<DownloadState>,
    pub condvar: Condvar,
}

impl StreamTask {
    pub fn downloaded(&self) -> u64 {
        if self.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
            self.total_size.load(Ordering::SeqCst)
        } else {
            self.downloaded.load(Ordering::SeqCst)
        }
    }

    pub fn set_failed(&self, message: &str) {
        self.status.store(STATUS_FAILED, Ordering::SeqCst);
        if let Ok(mut guard) = self.error.lock() {
            *guard = Some(message.to_string());
        }
        self.condvar.notify_all();
    }

    pub fn update_content_type(&self) {
        let mut guard = self.content_type.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            if let Some(content_type) = sniff_content_type(&self.part_path) {
                *guard = Some(content_type);
            }
        }
    }

    pub fn current_url(&self) -> Option<String> {
        let index = self.url_index.load(Ordering::SeqCst);
        self.audio_urls.get(index).cloned()
    }

    /// 切换到下一个备用 CDN 地址；无备用时返回 false。
    pub fn switch_to_next_url(&self) -> bool {
        let index = self.url_index.load(Ordering::SeqCst);
        if index + 1 < self.audio_urls.len() {
            self.url_index.store(index + 1, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub fn content_type(&self) -> String {
        let guard = self.content_type.lock().unwrap_or_else(|p| p.into_inner());
        guard.clone().unwrap_or_else(|| "audio/mp4".to_string())
    }

    pub fn set_jump(&self, position: u64) {
        let _ = self.jump_target.fetch_min(position, Ordering::SeqCst);
    }

    /// 将 [start, end] 写入已填充区间并通知等待中的读取端。
    pub fn add_filled(&self, start: u64, end: u64) {
        if end < start {
            return;
        }
        let prefix = {
            let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
            guard.filled.push((start, end));
            guard.filled.sort_unstable();
            let mut merged: Vec<(u64, u64)> = Vec::with_capacity(guard.filled.len());
            for &(s, e) in guard.filled.iter() {
                if let Some(last) = merged.last_mut() {
                    if s <= last.1.saturating_add(1) {
                        last.1 = last.1.max(e);
                        continue;
                    }
                }
                merged.push((s, e));
            }
            guard.filled = merged;
            prefix_len(&guard.filled)
        };
        self.downloaded.store(prefix, Ordering::SeqCst);
        self.condvar.notify_all();
    }

    /// 等待某个字节位置变为已填充；超时或任务结束返回 false。
    pub fn wait_filled(&self, position: u64, timeout: Duration) -> bool {
        let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let deadline = Instant::now() + timeout;
        loop {
            if ranges_contain(&guard.filled, position) {
                return true;
            }
            if self.cancelled.load(Ordering::SeqCst)
                || self.status.load(Ordering::SeqCst) != STATUS_DOWNLOADING
            {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let (next, _) = self
                .condvar
                .wait_timeout(guard, deadline - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard = next;
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatus {
    pub stream_id: String,
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub audio_id: u64,
    pub total_size: u64,
    pub downloaded: u64,
    pub status: String,
    pub error: Option<String>,
    pub cache_path: Option<String>,
    pub cache_percent: u8,
    pub port: u16,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn part_open_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(windows)]
    options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    options
}

/// 打开 .part 供读取（允许多读 + 下载器同时写）。
pub fn open_part_read(path: &Path) -> std::io::Result<fs::File> {
    part_open_options().read(true).open(path)
}

fn open_part_write(path: &Path) -> std::io::Result<fs::File> {
    part_open_options()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
}

/// 根据真实文件头判定媒体 Content-Type。
fn sniff_content_type(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut head = [0u8; 12];
    let n = file.read(&mut head).ok()?;
    if n >= 4 && &head[..4] == b"ftyp" {
        return Some("audio/mp4".to_string());
    }
    if n >= 4 && &head[..4] == b"OggS" {
        return Some("audio/ogg".to_string());
    }
    if n >= 4 && head[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return Some("audio/webm".to_string());
    }
    None
}

fn sidecar_path(part_path: &Path) -> PathBuf {
    let mut name = part_path.as_os_str().to_owned();
    name.push(".json");
    PathBuf::from(name)
}

#[derive(Serialize, Deserialize)]
struct PartSidecar {
    total: u64,
    ranges: Vec<(u64, u64)>,
}

fn save_sidecar(part_path: &Path, total: u64, ranges: &[(u64, u64)]) {
    let side = PartSidecar {
        total,
        ranges: ranges.to_vec(),
    };
    if let Ok(json) = serde_json::to_string(&side) {
        let _ = fs::write(sidecar_path(part_path), json);
    }
}

fn load_sidecar(part_path: &Path) -> Option<PartSidecar> {
    let json = fs::read_to_string(sidecar_path(part_path)).ok()?;
    serde_json::from_str(&json).ok()
}

fn prefix_len(filled: &[(u64, u64)]) -> u64 {
    match filled.first() {
        Some(&(start, end)) if start == 0 => end.saturating_add(1),
        _ => 0,
    }
}

fn ranges_contain(filled: &[(u64, u64)], position: u64) -> bool {
    filled
        .binary_search_by(|&(start, end)| {
            if position < start {
                std::cmp::Ordering::Greater
            } else if position > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub(crate) fn filled_end_at(filled: &[(u64, u64)], position: u64) -> Option<u64> {
    filled
        .iter()
        .find(|&&(start, end)| position >= start && position <= end)
        .map(|&(_, end)| end)
}

fn first_missing(filled: &[(u64, u64)], total: u64) -> Option<u64> {
    let mut position = 0u64;
    for &(start, end) in filled {
        if position < start {
            return Some(position);
        }
        position = position.max(end.saturating_add(1));
        if position >= total {
            return None;
        }
    }
    if position < total {
        Some(position)
    } else {
        None
    }
}

fn is_complete(filled: &[(u64, u64)], total: u64) -> bool {
    total > 0 && first_missing(filled, total).is_none()
}

/// 探测 CDN 音频总大小（Range 0-0 的 Content-Range）。
fn probe_total_with_client(client: &reqwest::blocking::Client, url: &str) -> u64 {
    if let Ok(response) = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
    {
        if let Some(value) = response.headers().get(reqwest::header::CONTENT_RANGE) {
            if let Ok(text) = value.to_str() {
                if let Some(total) = text.rsplit('/').next() {
                    if let Ok(total) = total.trim().parse::<u64>() {
                        return total;
                    }
                }
            }
        }
    }
    0
}

fn parse_total_from_response(response: &reqwest::blocking::Response) -> u64 {
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.rsplit('/').next()?.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// 后台下载器：顺序填充 .part，支持 seek 跳转与间隙回填，完成后原子重命名。
fn run_downloader(task: &Arc<StreamTask>) -> Result<(), ApiError> {
    let mut initial_filled: Vec<(u64, u64)> = Vec::new();
    let mut total = 0u64;

    // 恢复续传状态（边车文件优先，其次按顺序文件大小推断）。
    if let Some(side) = load_sidecar(&task.part_path) {
        initial_filled = side.ranges;
        total = side.total;
        task.total_size.store(total, Ordering::SeqCst);
    }
    if initial_filled.is_empty() {
        let size = fs::metadata(&task.part_path).map(|meta| meta.len()).unwrap_or(0);
        if size > 0 {
            initial_filled.push((0, size - 1));
        }
    }
    // 将恢复的区间写入任务状态，读取端才能立即读到已有数据。
    {
        let mut guard = task.state.lock().unwrap_or_else(|p| p.into_inner());
        guard.filled = initial_filled.clone();
    }
    let mut cursor = prefix_len(&initial_filled);
    task.downloaded.store(cursor, Ordering::SeqCst);
    task.update_content_type();

    let client = api::blocking_client();
    let Some(first_url) = task.current_url() else {
        return Ok(());
    };
    if total == 0 {
        total = probe_total_with_client(&client, &first_url);
        task.total_size.store(total, Ordering::SeqCst);
    }

    let mut gaps: Vec<(u64, u64)> = Vec::new();
    let mut bytes_since_save = 0u64;

    fn filled_snapshot(task: &Arc<StreamTask>) -> Vec<(u64, u64)> {
        task.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .filled
            .clone()
    }

    loop {
        if task.cancelled.load(Ordering::SeqCst) {
            save_sidecar(&task.part_path, total, &filled_snapshot(task));
            return Ok(());
        }

        // 跳过已填充区域（续传 / 跳转回填后避免重复下载）。
        loop {
            let filled = filled_snapshot(task);
            if let Some(end) = filled_end_at(&filled, cursor) {
                cursor = end.saturating_add(1);
                continue;
            }
            break;
        }

        // seek 跳转：读取端请求了更靠后的位置。
        let jump = task.jump_target.load(Ordering::SeqCst);
        if jump != u64::MAX && jump > cursor.saturating_add(JUMP_THRESHOLD) {
            if jump > cursor {
                gaps.push((cursor, jump - 1));
            }
            cursor = jump;
            let _ = task
                .jump_target
                .compare_exchange(jump, u64::MAX, Ordering::SeqCst, Ordering::SeqCst);
        }

        if total > 0 && cursor >= total {
            if let Some(gap) = gaps.pop() {
                cursor = gap.0;
                continue;
            }
            let filled = filled_snapshot(task);
            if let Some(missing) = first_missing(&filled, total) {
                cursor = missing;
                continue;
            }
            if is_complete(&filled, total) {
                finalize_task(task);
                if task.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
                    let _ = fs::remove_file(sidecar_path(&task.part_path));
                    return Ok(());
                }
                // 重命名失败（例如句柄未释放）则稍后重试。
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            return Ok(());
        }

        let end = if total > 0 {
            (cursor + CHUNK_SIZE - 1).min(total - 1)
        } else {
            cursor + CHUNK_SIZE - 1
        };
        let range = format!("bytes={cursor}-{end}");
        let Some(url) = task.current_url() else {
            return Ok(());
        };
        let response = match client
            .get(&url)
            .header(reqwest::header::RANGE, range.clone())
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                if task.switch_to_next_url() {
                    continue;
                }
                return Err(ApiError::Network(error));
            }
        };
        if !response.status().is_success() {
            let status = response.status();
            if task.switch_to_next_url() {
                continue;
            }
            return Err(ApiError::Http {
                status: status.as_u16(),
                body: format!("CDN 下载失败（Range {range}）"),
            });
        }
        if total == 0 {
            total = parse_total_from_response(&response);
            task.total_size.store(total, Ordering::SeqCst);
        }

        let is_partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut body = response;
        let mut file = open_part_write(&task.part_path)?;
        let mut buffer = [0u8; 64 * 1024];
        let mut written_this = 0u64;
        loop {
            let n = body.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            file.seek(SeekFrom::Start(cursor))?;
            file.write_all(&buffer[..n])?;
            let segment_start = cursor;
            cursor += n as u64;
            written_this += n as u64;
            task.add_filled(segment_start, cursor - 1);
            task.update_content_type();
            bytes_since_save += n as u64;
            if bytes_since_save >= 1024 * 1024 {
                bytes_since_save = 0;
                save_sidecar(&task.part_path, total, &filled_snapshot(task));
            }
            if task.cancelled.load(Ordering::SeqCst) {
                save_sidecar(&task.part_path, total, &filled_snapshot(task));
                return Ok(());
            }
        }
        // 请求区间未写满即 EOF：说明到达文件尾（total 未知时）。
        if total == 0 && (!is_partial || written_this < CHUNK_SIZE) {
            total = cursor;
            task.total_size.store(total, Ordering::SeqCst);
        }
        if written_this == 0 && total == 0 {
            return Err(ApiError::Invalid("CDN 返回了空数据".to_string()));
        }
    }
}

/// 创建流任务：写入 tracks 表、启动后台下载器、后台下载封面。
pub fn start_stream_task(
    detail: &VideoDetail,
    cid: u64,
    stream: &AudioStream,
    cache_root: &Path,
) -> Result<Arc<StreamTask>, ApiError> {
    fs::create_dir_all(cache_root)?;
    let (cache_path, part_path) =
        cache::track_paths(cache_root, &detail.bvid, cid, stream.audio_id, &stream.codec);
    let id = format!("s{}_{}", now_ms(), rand::random::<u32>());

    let task = Arc::new(StreamTask {
        id: id.clone(),
        bvid: detail.bvid.clone(),
        cid,
        title: detail.title.clone(),
        audio_urls: {
            let mut urls = vec![stream.url.clone()];
            urls.extend(stream.backup_urls.iter().cloned());
            urls
        },
        url_index: AtomicUsize::new(0),
        audio_id: stream.audio_id,
        codec: stream.codec.clone(),
        content_type: Mutex::new(None),
        total_size: AtomicU64::new(0),
        part_path,
        cache_path,
        status: AtomicU8::new(STATUS_DOWNLOADING),
        error: Mutex::new(None),
        cancelled: Arc::new(AtomicBool::new(false)),
        downloaded: AtomicU64::new(0),
        jump_target: AtomicU64::new(u64::MAX),
        state: Mutex::new(DownloadState::default()),
        condvar: Condvar::new(),
    });

    let track = db::tracks::TrackRecord {
        bvid: detail.bvid.clone(),
        cid,
        title: detail.title.clone(),
        cover_url: detail.cover.clone(),
        author: detail.author.clone(),
        duration: detail.duration,
        audio_id: stream.audio_id,
        codec: stream.codec.clone(),
        cache_path: Some(task.cache_path.to_string_lossy().into_owned()),
        cached_at: None,
    };
    {
        let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
        db::tracks::upsert_track(&connection, &track)?;
    }

    let cover_root = cache_root.to_path_buf();
    let cover_bvid = detail.bvid.clone();
    let cover_url = detail.cover.clone();
    std::thread::spawn(move || {
        let _ = cache::ensure_cover(&cover_root, &cover_bvid, &cover_url);
    });

    let download_task = task.clone();
    std::thread::spawn(move || {
        if let Err(error) = run_downloader(&download_task) {
            if !download_task.cancelled.load(Ordering::SeqCst) {
                download_task.set_failed(&format!("下载失败：{error}"));
            }
        }
    });

    manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(id, task.clone());
    Ok(task)
}

/// 已完整缓存的曲目：直接创建本地完成态任务，不联网、不重新下载。
pub fn try_start_cached(bvid: &str, cid: u64) -> Result<Option<Arc<StreamTask>>, ApiError> {
    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    let Some(track) = db::tracks::get_track_by_bvid(&connection, bvid, cid)? else {
        return Ok(None);
    };
    let Some(cache_path) = track.cache_path else {
        return Ok(None);
    };
    let cache_path = PathBuf::from(&cache_path);
    if !cache_path.exists() || track.cached_at.is_none() {
        return Ok(None);
    }
    let size = std::fs::metadata(&cache_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let id = format!("s{}_{}", now_ms(), rand::random::<u32>());
    let mut filled = Vec::new();
    if size > 0 {
        filled.push((0, size - 1));
    }
    let task = Arc::new(StreamTask {
        id: id.clone(),
        bvid: track.bvid.clone(),
        cid: track.cid,
        title: track.title.clone(),
        audio_urls: Vec::new(),
        url_index: AtomicUsize::new(0),
        audio_id: track.audio_id,
        codec: track.codec,
        content_type: Mutex::new(None),
        total_size: AtomicU64::new(size),
        part_path: cache_path.clone(),
        cache_path,
        status: AtomicU8::new(STATUS_COMPLETED),
        error: Mutex::new(None),
        cancelled: Arc::new(AtomicBool::new(false)),
        downloaded: AtomicU64::new(size),
        jump_target: AtomicU64::new(u64::MAX),
        state: Mutex::new(DownloadState { filled }),
        condvar: Condvar::new(),
    });
    manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(id, task.clone());
    task.update_content_type();
    Ok(Some(task))
}

pub fn get_task(id: &str) -> Option<Arc<StreamTask>> {
    manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(id)
        .cloned()
}

pub fn cancel_stream_task(id: &str) -> Result<(), ApiError> {
    let task = manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(id);
    if let Some(task) = task {
        task.cancelled.store(true, Ordering::SeqCst);
        task.status.store(STATUS_CANCELLED, Ordering::SeqCst);
        task.condvar.notify_all();
    }
    Ok(())
}

/// 当前任务使用的文件路径（正在播放/下载，清理时跳过）。
pub fn active_paths() -> Vec<PathBuf> {
    manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .values()
        .flat_map(|task| {
            vec![
                task.part_path.clone(),
                task.cache_path.clone(),
                sidecar_path(&task.part_path),
            ]
        })
        .collect()
}

/// 退出前清理：取消全部任务并删除 .part 临时文件与边车文件。
pub fn shutdown() {
    let tasks: Vec<Arc<StreamTask>> = manager()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .drain()
        .map(|(_, task)| task)
        .collect();
    for task in tasks {
        task.cancelled.store(true, Ordering::SeqCst);
        task.status.store(STATUS_CANCELLED, Ordering::SeqCst);
        task.condvar.notify_all();
        let _ = fs::remove_file(&task.part_path);
        let _ = fs::remove_file(sidecar_path(&task.part_path));
    }
}

/// 下载完成（全部区间填满）后重命名并同步 DB。
pub fn finalize_task(task: &Arc<StreamTask>) {
    if task.status.load(Ordering::SeqCst) != STATUS_DOWNLOADING {
        return;
    }
    let total = task.total_size.load(Ordering::SeqCst);
    if cache::finalize_part(&task.part_path, &task.cache_path, total) {
        task.status.store(STATUS_COMPLETED, Ordering::SeqCst);
        task.downloaded.store(total, Ordering::SeqCst);
        task.condvar.notify_all();
        let path = task.cache_path.to_string_lossy().into_owned();
        let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
        let _ = db::tracks::mark_cached(
            &connection,
            &task.bvid,
            task.cid,
            &path,
            now_secs() as i64,
        );
    }
}

pub fn stream_status(task: &Arc<StreamTask>, port: u16) -> StreamStatus {
    let status_code = task.status.load(Ordering::SeqCst);
    let status = match status_code {
        STATUS_COMPLETED => "completed",
        STATUS_CANCELLED => "cancelled",
        STATUS_FAILED => "failed",
        _ => "downloading",
    };
    let total = task.total_size.load(Ordering::SeqCst);
    let downloaded = task.downloaded();
    let percent = if total > 0 {
        ((downloaded * 100) / total).min(100) as u8
    } else {
        0
    };
    StreamStatus {
        stream_id: task.id.clone(),
        bvid: task.bvid.clone(),
        cid: task.cid,
        title: task.title.clone(),
        audio_id: task.audio_id,
        total_size: total,
        downloaded,
        status: status.to_string(),
        error: task.error.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        cache_path: if task.cache_path.exists() {
            Some(task.cache_path.to_string_lossy().into_owned())
        } else {
            None
        },
        cache_percent: percent,
        port,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_content_type_detects_containers() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-sniff-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("临时目录应可创建");

        let mp4 = dir.join("a.m4a");
        std::fs::write(&mp4, b"ftypmp42isom").expect("写文件应成功");
        assert_eq!(sniff_content_type(&mp4).as_deref(), Some("audio/mp4"));

        let ogg = dir.join("b.ogg");
        std::fs::write(&ogg, b"OggS\x00\x02").expect("写文件应成功");
        assert_eq!(sniff_content_type(&ogg).as_deref(), Some("audio/ogg"));

        let webm = dir.join("c.webm");
        std::fs::write(&webm, [0x1A, 0x45, 0xDF, 0xA3, 0x01, 0x00])
            .expect("写文件应成功");
        assert_eq!(sniff_content_type(&webm).as_deref(), Some("audio/webm"));

        let unknown = dir.join("d.bin");
        std::fs::write(&unknown, b"hello").expect("写文件应成功");
        assert_eq!(sniff_content_type(&unknown), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
