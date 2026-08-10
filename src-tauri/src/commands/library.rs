use crate::db;
use crate::db::favorites::FavoriteItem;
use crate::db::history::HistoryItem;
use crate::db::playlists::{PlaylistDetail, PlaylistSummary};
use crate::db::tracks::TrackInfo;
use crate::models::ApiError;

fn with_conn<T>(
    operation: impl FnOnce(&rusqlite::Connection) -> Result<T, ApiError>,
) -> Result<T, String> {
    let guard = db::connection()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&guard).map_err(|error| error.to_string())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn add_favorite(bvid: String, cid: u64) -> Result<(), String> {
    with_conn(|connection| db::favorites::add_favorite(connection, &bvid, cid, now_secs()))
}

#[tauri::command]
pub fn remove_favorite(bvid: String, cid: u64) -> Result<(), String> {
    with_conn(|connection| db::favorites::remove_favorite(connection, &bvid, cid))
}

#[tauri::command]
pub fn list_favorites() -> Result<Vec<FavoriteItem>, String> {
    with_conn(db::favorites::list_favorites)
}

#[tauri::command]
pub fn create_playlist(name: String) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("歌单名称不能为空".to_string());
    }
    with_conn(|connection| db::playlists::create_playlist(connection, name, now_secs()))
}

#[tauri::command]
pub fn rename_playlist(id: i64, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("歌单名称不能为空".to_string());
    }
    with_conn(|connection| db::playlists::rename_playlist(connection, id, name))
}

#[tauri::command]
pub fn delete_playlist(id: i64) -> Result<(), String> {
    with_conn(|connection| db::playlists::delete_playlist(connection, id))
}

#[tauri::command]
pub fn list_playlists() -> Result<Vec<PlaylistSummary>, String> {
    with_conn(db::playlists::list_playlists)
}

#[tauri::command]
pub fn get_playlist(id: i64) -> Result<PlaylistDetail, String> {
    with_conn(|connection| {
        let name = db::playlists::get_playlist_name(connection, id)?
            .ok_or_else(|| ApiError::Invalid("歌单不存在".to_string()))?;
        let tracks = db::playlists::list_playlist_tracks(connection, id)?;
        Ok(PlaylistDetail { id, name, tracks })
    })
}

#[tauri::command]
pub fn add_to_playlist(playlist_id: i64, bvid: String, cid: u64) -> Result<(), String> {
    with_conn(|connection| db::playlists::add_track(connection, playlist_id, &bvid, cid))
}

#[tauri::command]
pub fn remove_from_playlist(playlist_id: i64, bvid: String, cid: u64) -> Result<(), String> {
    with_conn(|connection| db::playlists::remove_track(connection, playlist_id, &bvid, cid))
}

#[tauri::command]
pub fn move_in_playlist(
    playlist_id: i64,
    bvid: String,
    cid: u64,
    to_position: i64,
) -> Result<(), String> {
    with_conn(|connection| {
        db::playlists::move_track(connection, playlist_id, &bvid, cid, to_position)
    })
}

#[tauri::command]
pub fn add_history(bvid: String, cid: u64) -> Result<(), String> {
    with_conn(|connection| db::history::add_history(connection, &bvid, cid, now_secs()))
}

#[tauri::command]
pub fn list_history() -> Result<Vec<HistoryItem>, String> {
    with_conn(db::history::list_history)
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    with_conn(db::history::clear_history)
}

#[tauri::command]
pub fn list_cached_tracks() -> Result<Vec<TrackInfo>, String> {
    with_conn(db::tracks::list_cached_tracks)
}
