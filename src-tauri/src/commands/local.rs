use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::cache;
use crate::db;
use crate::local;
use crate::models::{LocalFolder, LocalScanResult, LocalTrack};

fn with_conn<T>(
    operation: impl FnOnce(&rusqlite::Connection) -> Result<T, rusqlite::Error>,
) -> Result<T, String> {
    let guard = db::connection()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&guard).map_err(|error| error.to_string())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cover_root(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(cache::cache_root(&data_dir).join("local-covers"))
}

#[tauri::command]
pub fn pick_local_folder() -> Result<Option<String>, String> {
    let dialog = rfd::FileDialog::new().set_title("选择音乐文件夹");
    Ok(dialog.pick_folder().map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn add_local_folder(
    app: AppHandle,
    path: String,
) -> Result<LocalFolder, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err("选择的路径不是文件夹".to_string());
    }
    let cover = cover_root(&app)?;
    let stored_path = path_buf.to_string_lossy().into_owned();
    let folder_id = with_conn(|conn| db::local::add_folder(conn, &stored_path, now_secs()))?;
    tauri::async_runtime::spawn_blocking(move || {
        let scan_conn = db::connection()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        local::scan_folder(&scan_conn, folder_id, &path_buf, &cover)
    })
    .await
    .map_err(|error| error.to_string())??;
    let folders = with_conn(db::local::list_folders)?;
    folders
        .into_iter()
        .find(|folder| folder.id == folder_id)
        .ok_or_else(|| "文件夹读取失败".to_string())
}

#[tauri::command]
pub fn list_local_folders() -> Result<Vec<LocalFolder>, String> {
    with_conn(db::local::list_folders)
}

#[tauri::command]
pub fn remove_local_folder(id: i64) -> Result<(), String> {
    with_conn(|conn| db::local::delete_folder(conn, id))
}

#[tauri::command]
pub async fn rescan_local_folder(
    app: AppHandle,
    id: i64,
) -> Result<LocalScanResult, String> {
    let folder = with_conn(|conn| db::local::get_folder(conn, id))?
        .ok_or_else(|| "文件夹不存在".to_string())?;
    let path_buf = PathBuf::from(&folder.path);
    let cover = cover_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let scan_conn = db::connection()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        local::scan_folder(&scan_conn, id, &path_buf, &cover)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn list_local_tracks(
    query: Option<String>,
    folder_id: Option<i64>,
) -> Result<Vec<LocalTrack>, String> {
    with_conn(|conn| db::local::list_tracks(conn, query.as_deref(), folder_id))
}

#[tauri::command]
pub fn remove_local_track(id: i64) -> Result<(), String> {
    with_conn(|conn| db::local::delete_track(conn, id))
}

#[tauri::command]
pub fn add_local_favorite(id: i64) -> Result<(), String> {
    with_conn(|conn| db::local::add_favorite(conn, id, now_secs()))
}

#[tauri::command]
pub fn remove_local_favorite(id: i64) -> Result<(), String> {
    with_conn(|conn| db::local::remove_favorite(conn, id))
}

#[tauri::command]
pub fn list_local_favorites() -> Result<Vec<LocalTrack>, String> {
    with_conn(db::local::list_favorites)
}

#[tauri::command]
pub fn add_local_history(id: i64) -> Result<(), String> {
    with_conn(|conn| db::local::add_history(conn, id, now_secs()))
}

#[tauri::command]
pub fn list_local_history() -> Result<Vec<LocalTrack>, String> {
    with_conn(db::local::list_history)
}

#[tauri::command]
pub fn clear_local_history() -> Result<(), String> {
    with_conn(db::local::clear_history)
}
