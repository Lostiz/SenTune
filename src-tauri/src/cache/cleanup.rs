use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::db;
use crate::models::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub total_size: u64,
    pub file_count: u64,
    pub capacity_limit_gb: i64,
    pub cache_path: String,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 扫描缓存目录：跳过 .part，统计音频与封面。
pub fn scan(cache_root: &Path) -> (u64, u64) {
    let mut total = 0u64;
    let mut count = 0u64;
    let mut stack = vec![cache_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.ends_with(".part"))
            {
                total = total.saturating_add(entry.metadata().map(|m| m.len()).unwrap_or(0));
                count += 1;
            }
        }
    }
    (total, count)
}

fn delete_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 清理超龄文件；容量超限时按 last_played_at 最旧优先淘汰。
/// 正在播放/下载（active）与保留期内的文件跳过。返回删除文件数。
pub fn cleanup(
    cache_root: &Path,
    keep_days: i64,
    capacity_limit_gb: i64,
    active: &HashSet<PathBuf>,
) -> Result<u64, ApiError> {
    let now = now_secs();
    let keep_secs = keep_days.saturating_mul(86_400);
    let mut deleted = 0u64;
    let mut deleted_paths: Vec<String> = Vec::new();
    let mut remaining: Vec<(String, Option<i64>)> = Vec::new();

    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    for entry in db::tracks::list_cache_entries(&connection)? {
        let path = PathBuf::from(&entry.cache_path);
        if !path.exists() || active.contains(&path) {
            continue;
        }
        let age_secs = entry
            .cached_at
            .map(|cached_at| now.saturating_sub(cached_at))
            .unwrap_or(0);
        if age_secs > keep_secs {
            delete_file(&path);
            deleted += 1;
            deleted_paths.push(entry.cache_path.clone());
        } else {
            remaining.push((entry.cache_path.clone(), entry.last_played_at));
        }
    }

    // 封面与孤儿 .part：按保留天数清理（封面用 mtime 近似）。
    let covers = cache_root.join("covers");
    if let Ok(entries) = std::fs::read_dir(&covers) {
        for entry in entries.flatten() {
            let path = entry.path();
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if now.saturating_sub(modified) > keep_secs {
                delete_file(&path);
                deleted += 1;
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(cache_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !name.ends_with(".part") || active.contains(&path) {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if now.saturating_sub(modified) > keep_secs {
                delete_file(&path);
                deleted += 1;
            }
        }
    }

    // 容量上限淘汰。
    let mut total = scan(cache_root).0;
    let limit_bytes = (capacity_limit_gb as u64).saturating_mul(1024 * 1024 * 1024);
    if capacity_limit_gb > 0 && total > limit_bytes {
        remaining.sort_by_key(|(_, last_played_at)| last_played_at.unwrap_or(i64::MIN));
        for (path, _) in remaining {
            if total <= limit_bytes {
                break;
            }
            let path_buf = PathBuf::from(&path);
            if !path_buf.exists() || active.contains(&path_buf) {
                continue;
            }
            if let Ok(meta) = std::fs::metadata(&path_buf) {
                total = total.saturating_sub(meta.len());
            }
            delete_file(&path_buf);
            deleted += 1;
            deleted_paths.push(path);
        }
    }

    db::tracks::clear_cache_entries(&connection, &deleted_paths)?;
    Ok(deleted)
}

/// 迁移缓存目录：移动全部文件并同步 DB 中的 cache_path。
pub fn migrate_cache_root(old_root: &Path, new_root: &Path) -> Result<(), ApiError> {
    if old_root == new_root {
        return Ok(());
    }
    if !old_root.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(new_root)?;
    let mut stack = vec![old_root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    for file in files {
        let relative = file.strip_prefix(old_root).unwrap_or(&file);
        let target = new_root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::rename(&file, &target);
    }
    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    let old_prefix = old_root.to_string_lossy().into_owned();
    let new_prefix = new_root.to_string_lossy().into_owned();
    db::tracks::update_cache_paths(&connection, &old_prefix, &new_prefix)?;
    let _ = std::fs::remove_dir_all(old_root);
    Ok(())
}

/// 供启动/每日定时任务调用的整段清理。
pub fn run_scheduled_cleanup(cache_root: &Path) -> Result<u64, ApiError> {
    let settings = {
        let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
        db::settings::load_cache_settings(&connection)?
    };
    let active: HashSet<PathBuf> = crate::stream::active_paths().into_iter().collect();
    cleanup(
        cache_root,
        settings.keep_days,
        settings.capacity_limit_gb,
        &active,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_counts_only_complete_files() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-cleanup-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("covers")).expect("目录应可创建");
        std::fs::write(dir.join("a.m4a"), vec![0u8; 10]).expect("写文件应成功");
        std::fs::write(dir.join("b.part"), vec![0u8; 99]).expect("写文件应成功");
        std::fs::write(dir.join("covers").join("c.jpg"), vec![0u8; 5])
            .expect("写文件应成功");
        let (total, count) = scan(&dir);
        assert_eq!(total, 15);
        assert_eq!(count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
