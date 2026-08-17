use tauri::{AppHandle, Manager};

use crate::api;
use crate::cache;
use crate::models::VideoDetail;
use crate::stream;

#[tauri::command]
pub async fn start_stream(
    app: AppHandle,
    bvid: String,
    cid: Option<u64>,
    exclude_audio_id: Option<u64>,
) -> Result<stream::StreamStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let cache_root = cache::cache_root(&data_dir);
    // 已完整缓存的曲目直接本地播放（无需联网、不重复缓存）。
    if let Some(requested_cid) = cid {
        if let Some(task) = stream::try_start_cached(&bvid, requested_cid)
            .map_err(|error| error.to_string())?
        {
            return Ok(stream::stream_status(&task, stream::server::local_port()));
        }
    }
    let detail = api::view::get_video_detail(&bvid)
        .await
        .map_err(|error| error.to_string())?;
    let cid = cid.unwrap_or(detail.cid);
    if cid != detail.cid && !detail.pages.iter().any(|page| page.cid == cid) {
        return Err("指定的分 P 不存在".to_string());
    }
    if let Some(task) = stream::try_start_cached(&bvid, cid)
        .map_err(|error| error.to_string())?
    {
        return Ok(stream::stream_status(&task, stream::server::local_port()));
    }
    let audio = api::playurl::get_audio_stream(&bvid, cid, exclude_audio_id)
        .await
        .map_err(|error| error.to_string())?;
    let task = stream::start_stream_task(&detail, cid, &audio, &cache_root)
        .map_err(|error| error.to_string())?;
    Ok(stream::stream_status(&task, stream::server::local_port()))
}

#[tauri::command]
pub async fn get_video_detail(bvid: String) -> Result<VideoDetail, String> {
    api::view::get_video_detail(&bvid)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cancel_stream(stream_id: String) -> Result<(), String> {
    stream::cancel_stream_task(&stream_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_stream_status(stream_id: String) -> Result<stream::StreamStatus, String> {
    let task = stream::get_task(&stream_id).ok_or_else(|| "流任务不存在".to_string())?;
    Ok(stream::stream_status(&task, stream::server::local_port()))
}

#[tauri::command]
pub async fn get_cover_url(app: AppHandle, bvid: String) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let path = cache::cover_path(&cache::cache_root(&data_dir), &bvid);
    if path.exists() {
        Ok(format!(
            "http://127.0.0.1:{}/cover/{bvid}.jpg",
            stream::server::local_port()
        ))
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub fn get_proxy_port() -> u16 {
    stream::server::local_port()
}
