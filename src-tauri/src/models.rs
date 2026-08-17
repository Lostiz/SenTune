use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoItem {
    pub bvid: String,
    pub title: String,
    pub pic: String,
    pub duration: u64,
    pub author: String,
    pub play: u64,
    pub danmaku: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub items: Vec<VideoItem>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDetail {
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub cover: String,
    pub duration: u64,
    pub author: String,
    pub play: u64,
    pub pages: Vec<VideoPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPage {
    pub cid: u64,
    pub page: u32,
    pub part: String,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrack {
    pub id: i64,
    pub path: String,
    pub folder_id: Option<i64>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: u64,
    pub codec: String,
    pub size: u64,
    pub modified_at: i64,
    pub cover_path: Option<String>,
    pub added_at: i64,
    pub last_played_at: Option<i64>,
    pub play_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFolder {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
    pub track_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalScanResult {
    pub folder_id: i64,
    pub path: String,
    pub added: u64,
    pub updated: u64,
    pub removed: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStream {
    pub url: String,
    pub backup_urls: Vec<String>,
    pub audio_id: u64,
    pub codec: String,
    pub bandwidth: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseSong {
    pub id: u64,
    pub name: String,
    pub artist: String,
    pub album_name: String,
    pub pic_url: String,
    pub duration_ms: u64,
    /// 网易云付费标记：0 免费可播，非 0 为 VIP/付费（无登录不可播）。
    pub fee: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseSearchPage {
    pub items: Vec<NeteaseSong>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseStream {
    pub url: String,
    pub bitrate: u64,
    pub codec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseLyric {
    pub song_id: u64,
    pub lyric: Option<String>,
    pub translated_lyric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseArtist {
    pub id: u64,
    pub name: String,
    pub pic_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseArtistSearchPage {
    pub items: Vec<NeteaseArtist>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseArtistDetail {
    pub id: u64,
    pub name: String,
    pub pic_url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseAlbum {
    pub id: u64,
    pub name: String,
    pub pic_url: String,
    pub artist: String,
    pub publish_time: i64,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseAlbumDetail {
    pub album: NeteaseAlbum,
    pub songs: Vec<NeteaseSong>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("网络请求失败：{0}")]
    Network(#[from] reqwest::Error),
    #[error("HTTP {status}：{body}")]
    Http { status: u16, body: String },
    #[error("B 站接口错误（{code}）：{message}")]
    Bili { code: i64, message: String },
    #[error("网易云接口错误（{code}）：{message}")]
    Netease { code: i64, message: String },
    #[error("未登录或登录已过期")]
    Unauthorized,
    #[error("响应解析失败：{0}")]
    Parse(#[from] serde_json::Error),
    #[error("参数错误：{0}")]
    Invalid(String),
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据库错误：{0}")]
    Sql(#[from] rusqlite::Error),
}

pub(crate) fn normalize_url(raw: &str) -> String {
    if raw.starts_with("//") {
        format!("https:{raw}")
    } else {
        raw.to_string()
    }
}

pub(crate) fn strip_keyword_tags(title: &str) -> String {
    title
        .replace("<em class=\"keyword\">", "")
        .replace("</em>", "")
}
