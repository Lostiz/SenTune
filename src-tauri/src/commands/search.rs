use crate::api;
use crate::models::SearchPage;

#[tauri::command]
pub async fn search_videos(keyword: String, page: u32) -> Result<SearchPage, String> {
    api::search::search_videos(&keyword, page)
        .await
        .map_err(|error| error.to_string())
}
