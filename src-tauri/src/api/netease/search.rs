//! 网易云搜索：公开接口 `/api/cloudsearch/pc`（type=1 单曲，每页 10 条）。
//!
//! weapi 加密接口已失效，本接口无需加密、返回完整字段（含封面 picUrl）。

use serde::Deserialize;

use super::client::{api_get, RateKind};
use crate::models::{normalize_url, ApiError, NeteaseSearchPage, NeteaseSong};

const SEARCH_PATH: &str = "/api/cloudsearch/pc";
const PAGE_SIZE: u32 = 10;

#[derive(Deserialize)]
struct CloudSearchResponse {
    result: CloudSearchResult,
}

#[derive(Deserialize)]
struct CloudSearchResult {
    #[serde(default)]
    songs: Vec<CloudSong>,
    #[serde(rename = "songCount", default)]
    song_count: u64,
}

#[derive(Deserialize)]
struct CloudSong {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    ar: Vec<CloudArtist>,
    al: Option<CloudAlbum>,
    #[serde(default)]
    dt: u64,
    #[serde(default)]
    fee: u64,
}

#[derive(Deserialize)]
struct CloudArtist {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct CloudAlbum {
    #[serde(default)]
    name: String,
    #[serde(rename = "picUrl", default)]
    pic_url: String,
}

pub async fn search(keyword: &str, page: u32) -> Result<NeteaseSearchPage, ApiError> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(ApiError::Invalid("搜索关键词不能为空".to_string()));
    }
    let page = page.max(1);
    let offset = (page - 1) * PAGE_SIZE;
    let raw_params = vec![
        ("s".to_string(), keyword.to_string()),
        ("type".to_string(), "1".to_string()),
        ("limit".to_string(), PAGE_SIZE.to_string()),
        ("offset".to_string(), offset.to_string()),
    ];
    let params: Vec<(&str, &str)> = raw_params
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let value = api_get(SEARCH_PATH, &params, RateKind::Search).await?;
    let response: CloudSearchResponse = serde_json::from_value(value)?;

    let mut items = Vec::new();
    for song in response.result.songs {
        let name = song.name.trim();
        if name.is_empty() {
            continue;
        }
        let artist = song
            .ar
            .iter()
            .filter_map(|artist| {
                let name = artist.name.trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join(" / ");
        let album = song.al.unwrap_or_default();
        items.push(NeteaseSong {
            id: song.id,
            name: name.to_string(),
            artist,
            album_name: album.name,
            pic_url: normalize_url(&album.pic_url),
            duration_ms: song.dt,
            fee: song.fee,
        });
    }
    let total = response.result.song_count.max(items.len() as u64);
    let total_pages = total.div_ceil(PAGE_SIZE as u64).max(1) as u32;
    Ok(NeteaseSearchPage {
        items,
        page,
        page_size: PAGE_SIZE,
        total,
        total_pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cloudsearch_response() {
        let json = serde_json::json!({
            "code": 200,
            "result": {
                "songCount": 336,
                "songs": [
                    {
                        "id": 2652820720u64,
                        "name": "晴天(深情版)",
                        "ar": [{ "name": "Lucky小爱" }, { "name": "伴奏" }],
                        "al": { "name": "晴天(深情版)", "picUrl": "http://p2.music.126.net/abc/1.jpg" },
                        "dt": 278961,
                        "fee": 8
                    },
                    {
                        "id": 2636660086u64,
                        "name": "Luvsic（纯音乐）",
                        "ar": [{ "name": "含子逸" }],
                        "al": { "name": "释怀", "picUrl": "" },
                        "dt": 195394,
                        "fee": 0
                    }
                ]
            }
        });
        let response: CloudSearchResponse =
            serde_json::from_value(json).expect("响应应可解析");
        let songs = response.result.songs;
        assert_eq!(response.result.song_count, 336);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].id, 2652820720);
        assert_eq!(songs[0].ar.len(), 2);
        assert_eq!(songs[0].al.as_ref().expect("al 存在").pic_url, "http://p2.music.126.net/abc/1.jpg");
        assert_eq!(songs[0].fee, 8);
        assert_eq!(songs[1].fee, 0);
    }

    #[tokio::test]
    #[ignore = "需要真实网络访问网易云接口"]
    async fn live_search_returns_songs() {
        let page = search("周杰伦", 1).await.expect("搜索应成功");
        assert!(!page.items.is_empty(), "关键词「周杰伦」应返回歌曲");
        assert!(page.total >= page.items.len() as u64);
        assert_eq!(page.page_size, 10);
        for song in page.items.iter().take(3) {
            assert!(song.id > 0, "歌曲 id 非空");
            assert!(!song.name.is_empty(), "歌名非空");
            assert!(song.duration_ms > 0, "时长非空");
        }
        // 「纯音乐」应包含免费歌曲，且免费歌曲有封面与可解析的播放地址。
        let page = search("纯音乐", 1).await.expect("搜索应成功");
        let free = page
            .items
            .iter()
            .find(|song| song.fee == 0)
            .expect("结果中应有免费歌曲");
        assert!(
            !free.pic_url.is_empty(),
            "免费歌曲应返回封面（picUrl）"
        );
        let stream = crate::api::netease::get_play_url(free.id)
            .await
            .expect("免费歌曲应能解析播放地址");
        assert!(stream.url.starts_with("https://"), "CDN 地址应为 https");
        assert_eq!(stream.codec, "mp3");
        assert_eq!(stream.bitrate, 128_000);
    }
}
