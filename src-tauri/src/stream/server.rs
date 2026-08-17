use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use tiny_http::{Header, Request, Response, Server, StatusCode};

use super::{filled_end_at, get_task, open_part_read, StreamTask, STATUS_COMPLETED};
use crate::api;
use crate::cache;

static PORT: OnceLock<u16> = OnceLock::new();

/// 启动本地流代理（127.0.0.1 随机端口），返回端口号。
pub fn start(cache_root: PathBuf) -> io::Result<u16> {
    let server = Server::http("127.0.0.1:0")
        .map_err(|error| io::Error::other(error))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|addr| addr.port())
        .unwrap_or(0);
    let _ = PORT.set(port);
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let cache_root = cache_root.clone();
            std::thread::spawn(move || {
                let _ = handle(request, &cache_root);
            });
        }
    });
    Ok(port)
}

pub fn local_port() -> u16 {
    *PORT.get().unwrap_or(&0)
}

fn handle(request: Request, cache_root: &Path) -> io::Result<()> {
    let url = request.url().to_string();
    if url.starts_with("/cover/remote") {
        return handle_cover_remote(request);
    }
    if let Some(id) = url.strip_prefix("/local-cover/") {
        let id = id
            .split(['?', '/'])
            .next()
            .unwrap_or(id)
            .to_string();
        return handle_local_cover(request, &id);
    }
    if url.starts_with("/local") {
        return handle_local(request);
    }
    if let Some(id) = url.strip_prefix("/stream/") {
        let id = id
            .split(['?', '/'])
            .next()
            .unwrap_or(id)
            .to_string();
        return handle_stream(request, &id);
    }
    if let Some(bvid) = url.strip_prefix("/cover/") {
        let bvid = bvid
            .trim_end_matches(".jpg")
            .split(['?', '/'])
            .next()
            .unwrap_or(bvid)
            .to_string();
        return serve_cover(request, &bvid, cache_root);
    }
    request
        .respond(Response::empty(StatusCode(404)))
        .map_err(|error| io::Error::other(error))
}

fn handle_local(request: Request) -> io::Result<()> {
    let raw = request.url().to_string();
    let parsed = match reqwest::Url::parse(&format!("http://127.0.0.1{raw}")) {
        Ok(url) => url,
        Err(_) => {
            return request
                .respond(Response::empty(StatusCode(404)))
                .map_err(|error| io::Error::other(error));
        }
    };
    let Some(path) = parsed
        .query_pairs()
        .find(|(key, _)| key == "path")
        .map(|(_, value)| value.into_owned())
    else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    {
        let guard = crate::db::connection()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !crate::db::local::is_allowed_path(&guard, &path) {
            return request
                .respond(Response::empty(StatusCode(404)))
                .map_err(|error| io::Error::other(error));
        }
    }
    let file_path = PathBuf::from(&path);
    if !file_path.is_file() {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }
    let content_type = crate::local::content_type_for_path(&file_path);
    let total = std::fs::metadata(&file_path)
        .map_err(|error| io::Error::other(error))?
        .len();
    let range_header = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());
    if let Some(range) = range_header {
        let (start, end) = parse_range(&range, total);
        return serve_file(request, &file_path, start, end, total, content_type);
    }
    let file = File::open(&file_path)?;
    let headers = vec![
        header("Content-Type", content_type),
        header("Content-Length", total.to_string()),
        header("Accept-Ranges", "bytes"),
    ];
    let response = Response::new(StatusCode(200), headers, file, Some(total as usize), None);
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

fn handle_local_cover(request: Request, id: &str) -> io::Result<()> {
    let Ok(track_id) = id.parse::<i64>() else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    let cover = {
        let guard = crate::db::connection()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        crate::db::local::get_cover_path(&guard, track_id)
            .map_err(|error| io::Error::other(error))?
            .filter(|path| !path.is_empty())
    };
    let Some(cover) = cover else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    let cover_path = PathBuf::from(cover);
    if !cover_path.is_file() {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }
    let content_type = if cover_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
    {
        "image/png"
    } else {
        "image/jpeg"
    };
    let file = File::open(&cover_path)?;
    let len = file.metadata()?.len();
    let headers = vec![
        header("Content-Type", content_type),
        header("Content-Length", len.to_string()),
    ];
    let response = Response::new(StatusCode(200), headers, file, Some(len as usize), None);
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

/// 远程封面代理：带 UA/Referer 抓取 B 站封面，解决防盗链导致的“无法获取封面”。
fn handle_cover_remote(request: Request) -> io::Result<()> {
    let raw = request.url().to_string();
    let parsed = match reqwest::Url::parse(&format!("http://127.0.0.1{raw}")) {
        Ok(url) => url,
        Err(_) => {
            return request
                .respond(Response::empty(StatusCode(404)))
                .map_err(|error| io::Error::other(error));
        }
    };
    let Some(remote) = parsed
        .query_pairs()
        .find(|(key, _)| key == "url")
        .map(|(_, value)| value.into_owned())
    else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    let Ok(remote_url) = reqwest::Url::parse(&remote) else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    if remote_url.scheme() != "http" && remote_url.scheme() != "https" {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }

    let response = match api::blocking_client().get(remote_url).send() {
        Ok(response) => response,
        Err(_) => {
            return request
                .respond(Response::empty(StatusCode(404)))
                .map_err(|error| io::Error::other(error));
        }
    };
    if !response.status().is_success() {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = match response.bytes() {
        Ok(bytes) if !bytes.is_empty() => bytes,
        _ => {
            return request
                .respond(Response::empty(StatusCode(404)))
                .map_err(|error| io::Error::other(error));
        }
    };
    let headers = vec![
        header("Content-Type", &content_type),
        header("Content-Length", bytes.len().to_string()),
        header("Cache-Control", "public, max-age=86400"),
    ];
    let response = Response::new(
        StatusCode(200),
        headers,
        Cursor::new(bytes.to_vec()),
        Some(bytes.len()),
        None,
    );
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

fn header(name: &str, value: impl AsRef<str>) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_ref().as_bytes()).expect("构造头部应成功")
}

fn base_headers(content_type: &str) -> Vec<Header> {
    vec![
        header("Accept-Ranges", "bytes"),
        header("Content-Type", content_type),
    ]
}

fn parse_range(value: &str, total: u64) -> (u64, Option<u64>) {
    let Some(range) = value.strip_prefix("bytes=") else {
        return (0, None);
    };
    let Some((start, end)) = range.split_once('-') else {
        return (0, None);
    };
    if start.is_empty() {
        // 后缀范围 bytes=-N
        let count: u64 = end.trim().parse().unwrap_or(0);
        if total > 0 {
            return (total.saturating_sub(count), None);
        }
        return (0, None);
    }
    let start: u64 = start.trim().parse().unwrap_or(0);
    let end = if end.is_empty() {
        None
    } else {
        end.trim().parse::<u64>().ok()
    };
    (start, end)
}

fn serve_file(
    request: Request,
    path: &Path,
    start: u64,
    end: Option<u64>,
    total_hint: u64,
    content_type: &str,
) -> io::Result<()> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let total = if total_hint > 0 { total_hint } else { file_len };
    if total == 0 {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }
    let end = end.unwrap_or(total - 1).min(total - 1);
    let start = start.min(end);
    let len = end - start + 1;
    file.seek(SeekFrom::Start(start))?;
    let mut headers = base_headers(content_type);
    headers.push(header("Content-Range", format!("bytes {start}-{end}/{total}")));
    headers.push(header("Content-Length", len.to_string()));
    let response = Response::new(
        StatusCode(206),
        headers,
        file.take(len),
        Some(len as usize),
        None,
    );
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

fn serve_cover(request: Request, bvid: &str, cache_root: &Path) -> io::Result<()> {
    let path = cache::cover_path(cache_root, bvid);
    if !path.exists() {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }
    let file = File::open(&path)?;
    let len = file.metadata()?.len();
    let headers = vec![
        header("Content-Type", "image/jpeg"),
        header("Content-Length", len.to_string()),
    ];
    let response = Response::new(StatusCode(200), headers, file, Some(len as usize), None);
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

/// 增长文件读取器：只读取“已填充”区间，未填充时等待后台下载器。
struct GrowingFileReader {
    file: File,
    position: u64,
    end: u64,
    task: Arc<StreamTask>,
    deadline: Instant,
}

impl Read for GrowingFileReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position > self.end {
            return Ok(0);
        }
        let total = self.task.total_size.load(Ordering::SeqCst);
        if total > 0 && self.position >= total {
            return Ok(0);
        }
        loop {
            let available_end = {
                let guard = self.task.state.lock().unwrap_or_else(|p| p.into_inner());
                filled_end_at(&guard.filled, self.position)
            };
            if let Some(available_end) = available_end {
                let limit = (available_end - self.position + 1)
                    .min((self.end - self.position).saturating_add(1))
                    .min(buf.len() as u64) as usize;
                if limit == 0 {
                    return Ok(0);
                }
                let n = self.file.read(&mut buf[..limit])?;
                if n == 0 {
                    return Ok(0);
                }
                self.position += n as u64;
                return Ok(n);
            }
            let status = self.task.status.load(Ordering::SeqCst);
            if self.task.cancelled.load(Ordering::SeqCst)
                || status == super::STATUS_FAILED
                || status == super::STATUS_CANCELLED
            {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "流任务已结束",
                ));
            }
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "等待缓存数据超时",
                ));
            }
            self.task.set_jump(self.position);
            let _ = self.task.wait_filled(self.position, Duration::from_millis(100));
        }
    }
}

fn wait_for_total(task: &Arc<StreamTask>) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let total = task.total_size.load(Ordering::SeqCst);
        if total > 0 {
            return total;
        }
        if task.status.load(Ordering::SeqCst) != super::STATUS_DOWNLOADING {
            return 0;
        }
        if Instant::now() >= deadline {
            return 0;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 等待 .part 文件出现；若任务已完成则返回（由调用方重新走缓存分支）。
fn wait_for_part(task: &Arc<StreamTask>) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if task.part_path.exists() {
            return Ok(());
        }
        if task.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
            return Ok(());
        }
        if task.cancelled.load(Ordering::SeqCst)
            || task.status.load(Ordering::SeqCst) == super::STATUS_FAILED
            || task.status.load(Ordering::SeqCst) == super::STATUS_CANCELLED
        {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "缓存文件不可用",
            ));
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "缓存文件尚未创建",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn handle_stream(request: Request, id: &str) -> io::Result<()> {
    let Some(task) = get_task(id) else {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    };
    if task.cancelled.load(Ordering::SeqCst) {
        return request
            .respond(Response::empty(StatusCode(404)))
            .map_err(|error| io::Error::other(error));
    }

    let range_header = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());

    if task.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
        let total = task.total_size.load(Ordering::SeqCst);
        let (start, end) = parse_range(range_header.as_deref().unwrap_or(""), total);
        let content_type = task.content_type();
        return serve_file(request, &task.cache_path, start, end, total, &content_type);
    }
    if task.status.load(Ordering::SeqCst) == super::STATUS_FAILED {
        return request
            .respond(Response::empty(StatusCode(502)))
            .map_err(|error| io::Error::other(error));
    }

    let total = wait_for_total(&task);
    if task.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
        let total = task.total_size.load(Ordering::SeqCst);
        let (start, end) = parse_range(range_header.as_deref().unwrap_or(""), total);
        let content_type = task.content_type();
        return serve_file(request, &task.cache_path, start, end, total, &content_type);
    }
    let (start, end) = parse_range(range_header.as_deref().unwrap_or(""), total);
    let end = match end {
        Some(end) => end,
        None if total > 0 => total - 1,
        None => u64::MAX,
    };

    wait_for_part(&task)?;
    if task.status.load(Ordering::SeqCst) == STATUS_COMPLETED {
        let total = task.total_size.load(Ordering::SeqCst);
        let (start, end) = parse_range(range_header.as_deref().unwrap_or(""), total);
        let content_type = task.content_type();
        return serve_file(request, &task.cache_path, start, end, total, &content_type);
    }

    // 首字节未就绪时不启动响应，避免把“空/错误流”交给媒体元素。
    if !task.wait_filled(start, Duration::from_secs(10)) {
        return request
            .respond(Response::empty(StatusCode(503)))
            .map_err(|error| io::Error::other(error));
    }

    let mut file = open_part_read(&task.part_path)?;
    file.seek(SeekFrom::Start(start))?;
    let reader = GrowingFileReader {
        file,
        position: start,
        end,
        task: task.clone(),
        deadline: Instant::now() + super::READ_WAIT_TIMEOUT,
    };

    let content_type = task.content_type();
    let mut headers = base_headers(&content_type);
    let mut data_len = None;
    if total > 0 {
        let response_end = end.min(total - 1);
        headers.push(header(
            "Content-Range",
            format!("bytes {start}-{response_end}/{total}"),
        ));
        if end != u64::MAX {
            let length = response_end - start + 1;
            headers.push(header("Content-Length", length.to_string()));
            data_len = Some(length as usize);
        }
    }
    let response = Response::new(StatusCode(206), headers, reader, data_len, None);
    request
        .respond(response)
        .map_err(|error| io::Error::other(error))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tiny_http::Server;

    use crate::db;
    use crate::models::{AudioStream, VideoDetail};
    use crate::stream::{self, server};

    #[test]
    fn proxy_streams_ranges_appends_part_and_finalizes() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-proxy-test-{}",
            std::process::id()
        ));
        let cache_root = dir.join("cache");
        std::fs::create_dir_all(&cache_root).expect("临时目录应可创建");
        db::init(&dir.join("sentune.db")).expect("数据库应可初始化");

        // 模拟 CDN：支持 Range 的 1000 字节音频。
        let audio: Vec<u8> = (0..1000u16).map(|i| (i % 251) as u8).collect();
        let mock = Server::http("127.0.0.1:0").expect("模拟 CDN 应可启动");
        let mock_port = mock
            .server_addr()
            .to_ip()
            .map(|addr| addr.port())
            .expect("端口应可读");
        std::thread::spawn(move || {
            for request in mock.incoming_requests() {
                let range = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Range"))
                    .map(|h| h.value.as_str().to_string())
                    .unwrap_or_else(|| "bytes=0-".to_string());
                let (start, end) = if range.starts_with("bytes=") {
                    let inner = &range[6..];
                    let (s, e) = inner.split_once('-').unwrap_or((inner, ""));
                    (
                        s.trim().parse::<usize>().unwrap_or(0),
                        e.trim().parse::<usize>().unwrap_or(audio.len() - 1),
                    )
                } else {
                    (0, audio.len() - 1)
                };
                let end = end.min(audio.len() - 1);
                let body = audio[start..=end].to_vec();
                let headers = vec![
                    tiny_http::Header::from_bytes(
                        &b"Content-Range"[..],
                        format!("bytes {start}-{end}/{}", audio.len()).as_bytes(),
                    )
                    .expect("Content-Range 头部"),
                    tiny_http::Header::from_bytes(
                        &b"Content-Length"[..],
                        body.len().to_string().as_bytes(),
                    )
                    .expect("Content-Length 头部"),
                ];
                let response = tiny_http::Response::new(
                    tiny_http::StatusCode(206),
                    headers,
                    Cursor::new(body),
                    None,
                    None,
                );
                let _ = request.respond(response);
            }
        });

        let port = server::start(cache_root.clone()).expect("本地代理应可启动");
        let detail = VideoDetail {
            bvid: "BV1MOCKPROXY".to_string(),
            cid: 1,
            title: "Mock 曲目".to_string(),
            cover: String::new(),
            duration: 60,
            author: "Mock".to_string(),
            play: 0,
            pages: Vec::new(),
        };
        let audio_stream = AudioStream {
            url: format!("http://127.0.0.1:{mock_port}/audio"),
            backup_urls: Vec::new(),
            audio_id: 30280,
            codec: "mp4a".to_string(),
            bandwidth: 1000,
        };
        let task = stream::start_stream_task(&detail, detail.cid, &audio_stream, &cache_root)
            .expect("创建任务应成功");
        let base = format!("http://127.0.0.1:{port}/stream/{}", task.id);
        let client = reqwest::blocking::Client::new();

        let first = client
            .get(&base)
            .header(reqwest::header::RANGE, "bytes=0-499")
            .send()
            .expect("第一段请求应成功");
        assert_eq!(first.status().as_u16(), 206);
        assert_eq!(first.bytes().expect("读取正文").len(), 500);
        let part_size = std::fs::metadata(&task.part_path).map(|meta| meta.len());
        assert!(
            part_size.map(|size| size >= 500).unwrap_or(false) || task.cache_path.exists(),
            "下载器应持续写入 .part（或已提前完成缓存）"
        );

        let second = client
            .get(&base)
            .header(reqwest::header::RANGE, "bytes=500-999")
            .send()
            .expect("第二段请求应成功");
        assert_eq!(second.status().as_u16(), 206);
        assert_eq!(second.bytes().expect("读取正文").len(), 500);
        // 后台下载器完成时会原子重命名，轮询等待。
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !task.cache_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            task.cache_path.exists(),
            "完成后应生成正式缓存文件"
        );
        assert!(!task.part_path.exists(), ".part 应被重命名");
        assert_eq!(stream::stream_status(&task, port).status, "completed");

        // 重开应用后再次播放同一曲目：应命中本地完成态，不联网、不重新缓存。
        let restarted =
            stream::try_start_cached(&detail.bvid, detail.cid).expect("缓存命中应成功");
        let restarted = restarted.expect("已缓存曲目应返回本地任务");
        assert_eq!(
            restarted.status.load(std::sync::atomic::Ordering::SeqCst),
            super::super::STATUS_COMPLETED
        );
        assert!(restarted.audio_urls.is_empty(), "本地播放不应携带 CDN 地址");
        assert!(restarted.cache_path.exists());

        let third = client
            .get(&base)
            .header(reqwest::header::RANGE, "bytes=200-299")
            .send()
            .expect("缓存内范围请求应成功");
        assert_eq!(third.status().as_u16(), 206);
        assert_eq!(third.bytes().expect("读取正文").len(), 100);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proxy_serves_far_seek_while_downloading() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-jump-test-{}",
            std::process::id()
        ));
        let cache_root = dir.join("cache");
        std::fs::create_dir_all(&cache_root).expect("临时目录应可创建");
        db::init(&dir.join("sentune-jump.db")).expect("数据库应可初始化");

        let audio: Vec<u8> = (0..10_000u16).map(|i| (i % 251) as u8).collect();
        let mock = Server::http("127.0.0.1:0").expect("模拟 CDN 应可启动");
        let mock_port = mock
            .server_addr()
            .to_ip()
            .map(|addr| addr.port())
            .expect("端口应可读");
        std::thread::spawn(move || {
            for request in mock.incoming_requests() {
                let range = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Range"))
                    .map(|h| h.value.as_str().to_string())
                    .unwrap_or_else(|| "bytes=0-".to_string());
                let (start, end) = if range.starts_with("bytes=") {
                    let inner = &range[6..];
                    let (s, e) = inner.split_once('-').unwrap_or((inner, ""));
                    (
                        s.trim().parse::<usize>().unwrap_or(0),
                        e.trim().parse::<usize>().unwrap_or(audio.len() - 1),
                    )
                } else {
                    (0, audio.len() - 1)
                };
                let end = end.min(audio.len() - 1);
                let body = audio[start..=end].to_vec();
                let headers = vec![
                    tiny_http::Header::from_bytes(
                        &b"Content-Range"[..],
                        format!("bytes {start}-{end}/{}", audio.len()).as_bytes(),
                    )
                    .expect("Content-Range 头部"),
                    tiny_http::Header::from_bytes(
                        &b"Content-Length"[..],
                        body.len().to_string().as_bytes(),
                    )
                    .expect("Content-Length 头部"),
                ];
                let response = tiny_http::Response::new(
                    tiny_http::StatusCode(206),
                    headers,
                    Cursor::new(body),
                    None,
                    None,
                );
                let _ = request.respond(response);
            }
        });

        let port = server::start(cache_root.clone()).expect("本地代理应可启动");
        let detail = VideoDetail {
            bvid: "BV1JUMPTEST".to_string(),
            cid: 2,
            title: "跳转测试".to_string(),
            cover: String::new(),
            duration: 120,
            author: "Mock".to_string(),
            play: 0,
            pages: Vec::new(),
        };
        let audio_stream = AudioStream {
            url: format!("http://127.0.0.1:{mock_port}/audio"),
            backup_urls: Vec::new(),
            audio_id: 30280,
            codec: "mp4a".to_string(),
            bandwidth: 1000,
        };
        let task = stream::start_stream_task(&detail, detail.cid, &audio_stream, &cache_root)
            .expect("创建任务应成功");
        // 模拟用户立即拖到 8MB 附近。
        task.set_jump(8_000);

        let base = format!("http://127.0.0.1:{port}/stream/{}", task.id);
        let client = reqwest::blocking::Client::new();

        let far = client
            .get(&base)
            .header(reqwest::header::RANGE, "bytes=8000-8999")
            .send()
            .expect("远端范围请求应成功");
        assert_eq!(far.status().as_u16(), 206);
        assert_eq!(far.bytes().expect("读取正文").len(), 1000);

        let near = client
            .get(&base)
            .header(reqwest::header::RANGE, "bytes=0-999")
            .send()
            .expect("近端范围请求应成功");
        assert_eq!(near.status().as_u16(), 206);
        assert_eq!(near.bytes().expect("读取正文").len(), 1000);

        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !task.cache_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(task.cache_path.exists(), "跳转与回填后应生成完整缓存");
        assert_eq!(stream::stream_status(&task, port).status, "completed");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
