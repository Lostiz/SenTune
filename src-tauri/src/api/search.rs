use serde::Deserialize;

use super::device::ensure_bili_ticket;
use super::get_data;
use super::wbi::sign_params;
use crate::models::{normalize_url, strip_keyword_tags, ApiError, SearchPage, VideoItem};

const SEARCH_URL: &str = "https://api.bilibili.com/x/web-interface/wbi/search/type";

#[derive(Deserialize)]
struct SearchResultItem {
    #[serde(rename = "type")]
    item_type: String,
    bvid: Option<String>,
    title: Option<String>,
    pic: Option<String>,
    duration: Option<serde_json::Value>,
    author: Option<String>,
    play: Option<serde_json::Value>,
    danmaku: Option<serde_json::Value>,
}

fn to_u64(value: &serde_json::Value) -> u64 {
    match value {
        serde_json::Value::Number(number) => number.as_u64().unwrap_or(0),
        serde_json::Value::String(text) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// 将 "MM:SS" / "HH:MM:SS" 或数字秒转为秒数。
fn duration_to_seconds(value: Option<&serde_json::Value>) -> u64 {
    let Some(value) = value else {
        return 0;
    };
    if let Some(seconds) = value.as_u64() {
        return seconds;
    }
    let Some(text) = value.as_str() else {
        return 0;
    };
    let parts: Vec<&str> = text.split(':').collect();
    match parts.len() {
        2 => {
            let minutes: u64 = parts[0].trim().parse().unwrap_or(0);
            let seconds: u64 = parts[1].trim().parse().unwrap_or(0);
            minutes * 60 + seconds
        }
        3 => {
            let hours: u64 = parts[0].trim().parse().unwrap_or(0);
            let minutes: u64 = parts[1].trim().parse().unwrap_or(0);
            let seconds: u64 = parts[2].trim().parse().unwrap_or(0);
            hours * 3600 + minutes * 60 + seconds
        }
        _ => 0,
    }
}

#[derive(Deserialize)]
struct SearchData {
    #[serde(default)]
    result: Vec<SearchResultItem>,
    v_voucher: Option<String>,
    #[serde(rename = "numResults")]
    num_results: Option<u64>,
    #[serde(rename = "numPages")]
    num_pages: Option<u64>,
}

pub async fn search_videos(keyword: &str, page: u32) -> Result<SearchPage, ApiError> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(ApiError::Invalid("搜索关键词不能为空".to_string()));
    }
    // B 站风控：搜索接口要求携带设备指纹 Cookie + bili_ticket，否则返回 v_voucher 空结果。
    ensure_bili_ticket().await?;
    let mut params: Vec<(String, String)> = vec![
        ("search_type".to_string(), "video".to_string()),
        ("keyword".to_string(), keyword.to_string()),
        ("page".to_string(), page.to_string()),
        ("page_size".to_string(), "10".to_string()),
    ];
    sign_params(&mut params).await?;
    let refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();

    let data: SearchData = get_data(SEARCH_URL, &refs).await?;
    if data.v_voucher.is_some() {
        return Err(ApiError::Invalid(
            "搜索触发风控，请稍后重试".to_string(),
        ));
    }
    let mut items = Vec::with_capacity(data.result.len());
    for item in data.result {
        if item.item_type != "video" {
            continue;
        }
        let Some(bvid) = item.bvid else {
            continue;
        };
        let Some(title) = item.title else {
            continue;
        };
        let title = strip_keyword_tags(&title);
        if bvid.is_empty() || title.is_empty() {
            continue;
        }
        items.push(VideoItem {
            bvid,
            title,
            pic: normalize_url(&item.pic.unwrap_or_default()),
            duration: duration_to_seconds(item.duration.as_ref()),
            author: item.author.unwrap_or_default(),
            play: item.play.as_ref().map(to_u64).unwrap_or(0),
            danmaku: item.danmaku.as_ref().map(to_u64).unwrap_or(0),
        });
    }
    let total = data.num_results.unwrap_or(items.len() as u64);
    let total_pages = data.num_pages.unwrap_or(1).max(1) as u32;
    Ok(SearchPage {
        items,
        page,
        page_size: 10,
        total,
        total_pages,
    })
}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[test]
    fn search_data_parses_pagination_fields() {
        let json = r#"{
          "code": 0,
          "data": {
            "result": [],
            "numResults": 1000,
            "numPages": 50
          }
        }"#;
        let value: serde_json::Value =
            serde_json::from_str(json).expect("样例 JSON 应可解析");
        let data: SearchData =
            serde_json::from_value(value["data"].clone()).expect("SearchData 应可解析");
        assert_eq!(data.num_results, Some(1000));
        assert_eq!(data.num_pages, Some(50));
    }

    #[tokio::test]
    #[ignore = "需要真实网络访问 B 站接口"]
    async fn live_search_view_playurl_returns_valid_data() {
        let page = search_videos("纯音乐", 1).await.expect("搜索应成功");
        assert!(!page.items.is_empty(), "关键词「纯音乐」应至少返回 1 条");
        assert!(page.total_pages >= 1, "应返回总页数");
        assert_eq!(page.page_size, 10);
        for item in page.items.iter().take(5) {
            assert!(!item.title.is_empty(), "标题字段非空");
            assert!(!item.pic.is_empty(), "封面字段非空");
            assert!(item.duration > 0, "时长字段非空（>0 秒）");
        }
        let first = &page.items[0];
        let detail =
            crate::api::view::get_video_detail(&first.bvid).await.expect("详情应成功");
        assert!(detail.cid > 0, "cid 应非空");
        assert_eq!(detail.bvid, first.bvid);
        let stream =
            crate::api::playurl::get_audio_stream(&detail.bvid, detail.cid, None)
            .await
            .expect("playurl 应成功");
        assert!(stream.url.starts_with("http"), "音频 url 应为 http(s)");
        assert!(stream.audio_id >= 30216, "音频 id 应在普通 AAC 范围");
        assert!(stream.bandwidth > 0);
        // 真实 CDN Range 请求：确认 UA/Referer 下可返回 206 分片。
        let cdn_response = crate::api::http_client()
            .get(&stream.url)
            .header(reqwest::header::RANGE, "bytes=0-1023")
            .send()
            .await
            .expect("CDN 分片请求应成功");
        assert_eq!(
            cdn_response.status().as_u16(),
            206,
            "CDN 应支持 Range 并返回 206"
        );
        let body = cdn_response.bytes().await.expect("读取分片正文");
        assert!(!body.is_empty(), "分片正文非空");
    }
}
