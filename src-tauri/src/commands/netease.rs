use tauri::{AppHandle, Manager};

use crate::api;
use crate::cache;
use crate::db;
use crate::db::netease::{NeteaseFavoriteItem, NeteaseHistoryItem, NeteaseTrackRecord};
use crate::models::{
    ApiError, NeteaseAlbum, NeteaseAlbumDetail, NeteaseArtistDetail, NeteaseArtistSearchPage,
    NeteaseLyric, NeteaseSearchPage, NeteaseSong, NeteaseStream,
};
use crate::stream;

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

fn song_to_record(song: &NeteaseSong) -> NeteaseTrackRecord {
    NeteaseTrackRecord {
        song_id: song.id,
        title: song.name.clone(),
        artist: song.artist.clone(),
        album_name: song.album_name.clone(),
        cover_url: song.pic_url.clone(),
        duration_ms: song.duration_ms,
        fee: song.fee,
        cache_path: None,
        cached_at: None,
    }
}

#[tauri::command]
pub async fn search_netease(keyword: String, page: u32) -> Result<NeteaseSearchPage, String> {
    api::netease::search(&keyword, page)
        .await
        .map_err(|error| error.to_string())
}

/// 搜索网易云歌手。
#[tauri::command]
pub async fn search_netease_artists(
    keyword: String,
    page: u32,
) -> Result<NeteaseArtistSearchPage, String> {
    api::netease::search_artists(&keyword, page)
        .await
        .map_err(|error| error.to_string())
}

/// 获取网易云歌手详情。
#[tauri::command]
pub async fn get_netease_artist_detail(artist_id: u64) -> Result<NeteaseArtistDetail, String> {
    api::netease::get_artist_detail(artist_id)
        .await
        .map_err(|error| error.to_string())
}

/// 获取网易云歌手热门歌曲。
#[tauri::command]
pub async fn get_netease_artist_songs(artist_id: u64) -> Result<Vec<NeteaseSong>, String> {
    api::netease::get_artist_songs(artist_id)
        .await
        .map_err(|error| error.to_string())
}

/// 获取网易云歌手专辑列表。
#[tauri::command]
pub async fn get_netease_artist_albums(artist_id: u64) -> Result<Vec<NeteaseAlbum>, String> {
    api::netease::get_artist_albums(artist_id)
        .await
        .map_err(|error| error.to_string())
}

/// 获取网易云专辑详情（含歌曲列表）。
#[tauri::command]
pub async fn get_netease_album_detail(album_id: u64) -> Result<NeteaseAlbumDetail, String> {
    api::netease::get_album_detail(album_id)
        .await
        .map_err(|error| error.to_string())
}


/// 解析网易云播放地址（无登录 128kbps 档；URL 有时效，前端每次播放现取）。
#[tauri::command]
pub async fn netease_play_url(song_id: u64) -> Result<NeteaseStream, String> {
    api::netease::get_play_url(song_id)
        .await
        .map_err(|error| error.to_string())
}

/// 获取网易云歌词（原文 + 翻译，可能为空）。
#[tauri::command]
pub async fn netease_lyric(song_id: u64) -> Result<NeteaseLyric, String> {
    api::netease::get_lyric(song_id)
        .await
        .map_err(|error| error.to_string())
}

/// 启动网易云流：已缓存直接本地播放；否则解析播放地址后走边下边播管线。
#[tauri::command]
pub async fn start_netease_stream(
    app: AppHandle,
    song: NeteaseSong,
) -> Result<stream::StreamStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let cache_root = cache::cache_root(&data_dir);
    if let Some(task) = stream::try_start_cached_netease(song.id)
        .map_err(|error| error.to_string())?
    {
        return Ok(stream::stream_status(&task, stream::server::local_port()));
    }
    let stream = api::netease::get_play_url(song.id)
        .await
        .map_err(|error| error.to_string())?;
    let task = stream::start_netease_stream_task(&song, &stream, &cache_root)
        .map_err(|error| error.to_string())?;
    Ok(stream::stream_status(&task, stream::server::local_port()))
}

#[tauri::command]
pub fn add_netease_favorite(song: NeteaseSong) -> Result<(), String> {
    with_conn(|connection| {
        db::netease::add_favorite(connection, &song_to_record(&song), now_secs())
    })
}

#[tauri::command]
pub fn remove_netease_favorite(song_id: u64) -> Result<(), String> {
    with_conn(|connection| db::netease::remove_favorite(connection, song_id))
}

#[tauri::command]
pub fn list_netease_favorites() -> Result<Vec<NeteaseFavoriteItem>, String> {
    with_conn(db::netease::list_favorites)
}

#[tauri::command]
pub fn add_netease_history(song: NeteaseSong) -> Result<(), String> {
    with_conn(|connection| {
        db::netease::add_history(connection, &song_to_record(&song), now_secs())
    })
}

#[tauri::command]
pub fn list_netease_history() -> Result<Vec<NeteaseHistoryItem>, String> {
    with_conn(db::netease::list_history)
}

#[tauri::command]
pub fn clear_netease_history() -> Result<(), String> {
    with_conn(db::netease::clear_history)
}
