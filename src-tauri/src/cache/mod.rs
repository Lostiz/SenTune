use std::fs;
use std::path::{Path, PathBuf};

use crate::api;
use crate::models::ApiError;

pub mod cleanup;

pub fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache")
}

pub fn covers_dir(cache_root: &Path) -> PathBuf {
    cache_root.join("covers")
}

pub fn cover_path(cache_root: &Path, bvid: &str) -> PathBuf {
    covers_dir(cache_root).join(format!("{bvid}.jpg"))
}

pub fn extension_for_codec(codec: &str) -> &'static str {
    if codec == "opus" {
        "opus"
    } else {
        "m4a"
    }
}

/// 最终缓存文件与下载中 .part 文件路径。
pub fn track_paths(
    cache_root: &Path,
    bvid: &str,
    cid: u64,
    audio_id: u64,
    codec: &str,
) -> (PathBuf, PathBuf) {
    let ext = extension_for_codec(codec);
    let name = format!("{bvid}_{cid}_{audio_id}.{ext}");
    (cache_root.join(&name), cache_root.join(format!("{name}.part")))
}

/// 网易云缓存文件与 .part 路径（`netease_{songId}_{br}.mp3`）。
pub fn netease_track_paths(
    cache_root: &Path,
    song_id: u64,
    bitrate: u64,
) -> (PathBuf, PathBuf) {
    let name = format!("netease_{song_id}_{bitrate}.mp3");
    (cache_root.join(&name), cache_root.join(format!("{name}.part")))
}

pub fn part_size(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// 下载封面到 cache/covers/{bvid}.jpg（已存在则跳过）。
pub fn ensure_cover(cache_root: &Path, bvid: &str, url: &str) -> Result<(), ApiError> {
    let target = cover_path(cache_root, bvid);
    if target.exists() {
        return Ok(());
    }
    if url.is_empty() {
        return Ok(());
    }
    let response = api::blocking_client().get(url).send()?;
    if !response.status().is_success() {
        return Ok(());
    }
    let bytes = response.bytes()?;
    if bytes.is_empty() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, bytes)?;
    Ok(())
}

/// 校验并原子化完成缓存：.part → 正式文件。
pub fn finalize_part(part_path: &Path, cache_path: &Path, expected_size: u64) -> bool {
    let size = part_size(part_path);
    if expected_size > 0 && size >= expected_size {
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        return fs::rename(part_path, cache_path).is_ok();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_maps_codec() {
        assert_eq!(extension_for_codec("mp4a"), "m4a");
        assert_eq!(extension_for_codec("opus"), "opus");
        assert_eq!(extension_for_codec("unknown"), "m4a");
    }

    #[test]
    fn finalize_part_renames_when_complete() {
        let dir = std::env::temp_dir().join(format!("sentune-cache-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("临时目录应可创建");
        let part = dir.join("a.m4a.part");
        let cache = dir.join("a.m4a");
        fs::write(&part, vec![0u8; 16]).expect("写 .part 应成功");
        assert!(finalize_part(&part, &cache, 16));
        assert!(!part.exists(), ".part 应已被重命名");
        assert!(cache.exists(), "缓存文件应存在");
        assert!(!finalize_part(&part, &cache, 16), "重复调用应返回 false");
        let _ = fs::remove_dir_all(&dir);
    }
}
