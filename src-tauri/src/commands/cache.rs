use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::cache;
use crate::cache::cleanup::{CacheStatus, scan};
use crate::db;
use crate::db::settings::CacheSettings;
use crate::stream;

fn cache_root(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    let settings = db::settings::load_cache_settings(&connection)
        .map_err(|error| error.to_string())?;
    Ok(settings
        .cache_path
        .map(PathBuf::from)
        .unwrap_or_else(|| cache::cache_root(&data_dir)))
}

#[tauri::command]
pub fn get_cache_status(app: AppHandle) -> Result<CacheStatus, String> {
    let root = cache_root(&app)?;
    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    let settings = db::settings::load_cache_settings(&connection)
        .map_err(|error| error.to_string())?;
    let (total_size, file_count) = scan(&root);
    Ok(CacheStatus {
        total_size,
        file_count,
        capacity_limit_gb: settings.capacity_limit_gb,
        cache_path: root.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn clear_cache(
    app: AppHandle,
    older_than_days: Option<i64>,
) -> Result<CacheStatus, String> {
    let root = cache_root(&app)?;
    let settings = {
        let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
        db::settings::load_cache_settings(&connection).map_err(|error| error.to_string())?
    };
    let keep_days = match older_than_days {
        Some(0) => 0,
        Some(days) if days > 0 => days,
        _ => settings.keep_days,
    };
    let active: HashSet<PathBuf> = stream::active_paths().into_iter().collect();
    cache::cleanup::cleanup(&root, keep_days, settings.capacity_limit_gb, &active)
        .map_err(|error| error.to_string())?;
    let (total_size, file_count) = scan(&root);
    Ok(CacheStatus {
        total_size,
        file_count,
        capacity_limit_gb: settings.capacity_limit_gb,
        cache_path: root.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn get_cache_settings() -> Result<CacheSettings, String> {
    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    db::settings::load_cache_settings(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_cache_settings(
    keep_days: Option<i64>,
    capacity_limit_gb: Option<i64>,
    cache_path: Option<String>,
) -> Result<CacheSettings, String> {
    let mut settings = {
        let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
        db::settings::load_cache_settings(&connection).map_err(|error| error.to_string())?
    };

    if let Some(days) = keep_days {
        if days < 1 {
            return Err("保留天数至少为 1 天".to_string());
        }
        settings.keep_days = days;
    }
    if let Some(limit) = capacity_limit_gb {
        if limit != 0 && limit < db::settings::MIN_CAPACITY_GB {
            return Err(format!(
                "容量上限最低 {}GB（0 表示不限）",
                db::settings::MIN_CAPACITY_GB
            ));
        }
        settings.capacity_limit_gb = limit;
    }
    if let Some(path) = cache_path {
        let new_root = PathBuf::from(&path);
        if settings.cache_path.as_deref() != Some(path.as_str()) {
            if let Some(old) = settings.cache_path.take() {
                cache::cleanup::migrate_cache_root(Path::new(&old), &new_root)
                    .map_err(|error| error.to_string())?;
            }
            settings.cache_path = Some(path);
        }
    }

    let connection = db::connection().lock().unwrap_or_else(|p| p.into_inner());
    db::settings::save_cache_settings(&connection, &settings)
        .map_err(|error| error.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn pick_cache_dir() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("选择缓存目录")
        .pick_folder();
    Ok(picked.map(|path| path.to_string_lossy().into_owned()))
}
