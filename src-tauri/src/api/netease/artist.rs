//! 网易云歌手相关公开接口：歌手搜索、歌手详情、热门歌曲、专辑列表。
//!
//! 复用统一 UA / Referer / Cookie / 限流 / 熔断策略。

use serde_json::Value;

use super::client::{api_get, RateKind};
use crate::models::{
    normalize_url, ApiError, NeteaseAlbum, NeteaseAlbumDetail, NeteaseArtist,
    NeteaseArtistDetail, NeteaseArtistSearchPage, NeteaseSong,
};

const SEARCH_PATH: &str = "/api/cloudsearch/pc";
const ARTIST_INFO_PATH: &str = "/api/artist/head/info/get";
const ARTIST_SONGS_PATH: &str = "/api/artist/top/song";
const ARTIST_ALBUMS_PATH_PREFIX: &str = "/api/artist/albums";
const ALBUM_DETAIL_PATH: &str = "/api/album";
const SEARCH_PAGE_SIZE: u32 = 10;

fn parse_artist(value: &Value) -> Option<NeteaseArtist> {
    let id = value.get("id")?.as_u64()?;
    let name = value.get("name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    let pic_url = value
        .get("picUrl")
        .or_else(|| value.get("img1v1Url"))
        .and_then(Value::as_str)
        .map(normalize_url)
        .unwrap_or_default();
    Some(NeteaseArtist {
        id,
        name: name.to_string(),
        pic_url,
    })
}

fn parse_song(value: &Value) -> Option<NeteaseSong> {
    let id = value.get("id")?.as_u64()?;
    let name = value.get("name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    let artist = value
        .get("ar")
        .or_else(|| value.get("artists"))
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist.get("name").and_then(Value::as_str))
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .unwrap_or_default();
    let album = value
        .get("al")
        .or_else(|| value.get("album"));
    let album_name = album
        .and_then(|album| album.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let pic_url = album
        .and_then(|album| album.get("picUrl"))
        .and_then(Value::as_str)
        .map(normalize_url)
        .unwrap_or_default();
    let duration_ms = value.get("dt").and_then(Value::as_u64).unwrap_or(0);
    let fee = value.get("fee").and_then(Value::as_u64).unwrap_or(0);

    Some(NeteaseSong {
        id,
        name: name.to_string(),
        artist,
        album_name,
        pic_url,
        duration_ms,
        fee,
    })
}

fn parse_album(value: &Value) -> Option<NeteaseAlbum> {
    let id = value.get("id")?.as_u64()?;
    let name = value.get("name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    let artist = value
        .get("artist")
        .and_then(|artist| artist.get("name"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("artists")
                .and_then(Value::as_array)
                .and_then(|artists| artists.first())
                .and_then(|artist| artist.get("name"))
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .to_string();
    let pic_url = value
        .get("picUrl")
        .and_then(Value::as_str)
        .map(normalize_url)
        .unwrap_or_default();
    let publish_time = value.get("publishTime").and_then(Value::as_i64).unwrap_or(0);
    let size = value.get("size").and_then(Value::as_u64).unwrap_or(0) as u32;

    Some(NeteaseAlbum {
        id,
        name: name.to_string(),
        pic_url,
        artist,
        publish_time,
        size,
    })
}

/// 搜索歌手：`/api/cloudsearch/pc?type=100`。
pub async fn search_artists(
    keyword: &str,
    page: u32,
) -> Result<NeteaseArtistSearchPage, ApiError> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(ApiError::Invalid("搜索关键词不能为空".to_string()));
    }
    let page = page.max(1);
    let offset = (page - 1) * SEARCH_PAGE_SIZE;
    let id = keyword.to_string();
    let offset_str = offset.to_string();
    let limit = SEARCH_PAGE_SIZE.to_string();
    let params = vec![
        ("s", id.as_str()),
        ("type", "100"),
        ("limit", limit.as_str()),
        ("offset", offset_str.as_str()),
    ];
    let value = api_get(SEARCH_PATH, &params, RateKind::Search).await?;
    let result = value.get("result").cloned().unwrap_or_default();
    let items = result
        .get("artists")
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(parse_artist)
                .collect::<Vec<NeteaseArtist>>()
        })
        .unwrap_or_default();
    let total = result
        .get("artistCount")
        .and_then(Value::as_u64)
        .unwrap_or(items.len() as u64);
    let total_pages = total.div_ceil(SEARCH_PAGE_SIZE as u64).max(1) as u32;
    Ok(NeteaseArtistSearchPage {
        items,
        page,
        page_size: SEARCH_PAGE_SIZE,
        total,
        total_pages,
    })
}

/// 歌手详情：`/api/artist/head/info/get`。
pub async fn get_artist_detail(artist_id: u64) -> Result<NeteaseArtistDetail, ApiError> {
    if artist_id == 0 {
        return Err(ApiError::Invalid("歌手 ID 不能为空".to_string()));
    }
    let id = artist_id.to_string();
    let params = vec![("id", id.as_str())];
    let value = api_get(ARTIST_INFO_PATH, &params, RateKind::Playurl).await?;
    let artist = value.get("data").and_then(|data| data.get("artist"));

    let Some(artist) = artist else {
        return Err(ApiError::Netease {
            code: 0,
            message: "未找到歌手信息".to_string(),
        });
    };

    let id = artist.get("id").and_then(Value::as_u64).unwrap_or(artist_id);
    let name = artist
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("未知歌手")
        .to_string();
    let pic_url = artist
        .get("picUrl")
        .or_else(|| artist.get("cover"))
        .or_else(|| artist.get("img1v1Url"))
        .and_then(Value::as_str)
        .map(normalize_url)
        .unwrap_or_default();
    let description = artist
        .get("briefDesc")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|text| !text.is_empty());

    Ok(NeteaseArtistDetail {
        id,
        name,
        pic_url,
        description,
    })
}

/// 歌手热门歌曲：`/api/artist/top/song`。
pub async fn get_artist_songs(artist_id: u64) -> Result<Vec<NeteaseSong>, ApiError> {
    if artist_id == 0 {
        return Err(ApiError::Invalid("歌手 ID 不能为空".to_string()));
    }
    let id = artist_id.to_string();
    let params = vec![("id", id.as_str())];
    let value = api_get(ARTIST_SONGS_PATH, &params, RateKind::Playurl).await?;
    let songs = value
        .get("songs")
        .and_then(Value::as_array)
        .map(|songs| songs.iter().filter_map(parse_song).collect::<Vec<NeteaseSong>>())
        .unwrap_or_default();
    Ok(songs)
}

/// 歌手专辑列表：`/api/artist/albums/{id}`。
pub async fn get_artist_albums(artist_id: u64) -> Result<Vec<NeteaseAlbum>, ApiError> {
    if artist_id == 0 {
        return Err(ApiError::Invalid("歌手 ID 不能为空".to_string()));
    }
    let path = format!("{ARTIST_ALBUMS_PATH_PREFIX}/{artist_id}");
    let params = vec![("limit", "30"), ("offset", "0")];
    let value = api_get(&path, &params, RateKind::Playurl).await?;
    let albums = value
        .get("hotAlbums")
        .and_then(Value::as_array)
        .map(|albums| {
            albums
                .iter()
                .filter_map(parse_album)
                .collect::<Vec<NeteaseAlbum>>()
        })
        .unwrap_or_default();
    Ok(albums)
}


/// 专辑详情：`/api/album/{id}`，返回专辑信息和歌曲列表。
pub async fn get_album_detail(album_id: u64) -> Result<NeteaseAlbumDetail, ApiError> {
    if album_id == 0 {
        return Err(ApiError::Invalid("专辑 ID 不能为空".to_string()));
    }
    let path = format!("{ALBUM_DETAIL_PATH}/{album_id}");
    let params: [(&str, &str); 0] = [];
    let mut value = api_get(&path, &params, RateKind::Playurl).await?;

    // 部分网易云接口版本只支持 `?id=` 查询形式，这里做一次兜底。
    if value.get("album").is_none() || value.get("songs").is_none() {
        let id = album_id.to_string();
        let query_params = vec![("id", id.as_str())];
        value = api_get(ALBUM_DETAIL_PATH, &query_params, RateKind::Playurl).await?;
    }

    let album_value = value
        .get("album")
        .or_else(|| value.get("data").and_then(|data| data.get("album")))
        .cloned()
        .unwrap_or_default();
    let album = parse_album(&album_value).ok_or_else(|| ApiError::Netease {
        code: 0,
        message: "未找到专辑信息".to_string(),
    })?;
    let songs = value
        .get("songs")
        .or_else(|| value.get("tracks"))
        .or_else(|| value.get("album").and_then(|album| album.get("songs")))
        .or_else(|| value.get("data").and_then(|data| data.get("songs")))
        .and_then(Value::as_array)
        .map(|songs| songs.iter().filter_map(parse_song).collect::<Vec<NeteaseSong>>())
        .unwrap_or_default();

    Ok(NeteaseAlbumDetail { album, songs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_artist_from_cloudsearch() {
        let json = serde_json::json!({
            "id": 6452,
            "name": "周杰伦",
            "picUrl": "http://p1.music.126.net/abc/1.jpg",
            "albumSize": 30
        });
        let artist = parse_artist(&json).expect("应能解析歌手");
        assert_eq!(artist.id, 6452);
        assert_eq!(artist.name, "周杰伦");
        assert!(artist.pic_url.starts_with("https://"));
    }

    #[test]
    fn parses_song_from_artist_top() {
        let json = serde_json::json!({
            "id": 186016,
            "name": "七里香",
            "ar": [{ "name": "周杰伦" }],
            "al": { "name": "七里香", "picUrl": "http://p2.music.126.net/abc/2.jpg" },
            "dt": 300000,
            "fee": 0
        });
        let song = parse_song(&json).expect("应能解析歌曲");
        assert_eq!(song.id, 186016);
        assert_eq!(song.artist, "周杰伦");
        assert_eq!(song.album_name, "七里香");
    }

    #[test]
    fn parses_album_from_hot_albums() {
        let json = serde_json::json!({
            "id": 1900,
            "name": "七里香",
            "picUrl": "http://p1.music.126.net/abc/3.jpg",
            "artist": { "name": "周杰伦" },
            "publishTime": 1095344000000i64,
            "size": 10
        });
        let album = parse_album(&json).expect("应能解析专辑");
        assert_eq!(album.id, 1900);
        assert_eq!(album.artist, "周杰伦");
        assert_eq!(album.size, 10);
    }
}
