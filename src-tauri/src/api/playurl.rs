use serde::Deserialize;

use super::device::ensure_bili_ticket;
use super::get_data;
use super::wbi::sign_params;
use crate::models::{ApiError, AudioStream};

const PLAYURL_URL: &str = "https://api.bilibili.com/x/player/wbi/playurl";

#[derive(Deserialize)]
struct DashAudioItem {
    id: u64,
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(rename = "backupUrl")]
    backup_url: Option<Vec<String>>,
    codecs: String,
    bandwidth: u64,
}

#[derive(Deserialize)]
struct DashData {
    #[serde(default)]
    audio: Vec<DashAudioItem>,
}

#[derive(Deserialize)]
struct PlayurlData {
    dash: Option<DashData>,
}

/// 可播放优先：过滤特殊音质（Dolby / Hi-Res / 8D 等 id ≥ 30250 的非普通 AAC），
/// 优先选择 AAC，且音质从低到高（30216 → 30232 → 30280）；
/// 没有任何 AAC 时才选 opus 等备用项。
fn pick_best_audio(
    mut items: Vec<DashAudioItem>,
    exclude_audio_id: Option<u64>,
) -> Option<DashAudioItem> {
    items.retain(|item| {
        let is_special = item.id >= 30250 && item.codecs != "mp4a.40.2";
        !is_special
            && exclude_audio_id != Some(item.id)
            && (item.base_url.is_some()
                || !item.backup_url.as_ref().is_some_and(Vec::is_empty))
    });
    items.sort_by(|a, b| {
        let a_is_aac = a.codecs.contains("mp4a");
        let b_is_aac = b.codecs.contains("mp4a");
        b_is_aac
            .cmp(&a_is_aac)
            .then_with(|| a.id.cmp(&b.id))
    });
    items.into_iter().next()
}

/// 按 7.5 获取视频音频流，返回 AudioStream（url / audio_id / codec / bandwidth）。
pub async fn get_audio_stream(
    bvid: &str,
    cid: u64,
    exclude_audio_id: Option<u64>,
) -> Result<AudioStream, ApiError> {
    ensure_bili_ticket().await?;
    let mut params: Vec<(String, String)> = vec![
        ("bvid".to_string(), bvid.to_string()),
        ("cid".to_string(), cid.to_string()),
        ("fnval".to_string(), "16".to_string()),
        ("fnver".to_string(), "0".to_string()),
        ("fourk".to_string(), "1".to_string()),
    ];
    sign_params(&mut params).await?;
    let refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let data: PlayurlData = get_data(PLAYURL_URL, &refs).await?;
    let dash = data
        .dash
        .ok_or_else(|| ApiError::Invalid("playurl 响应缺少 dash".to_string()))?;
    let item = pick_best_audio(dash.audio, exclude_audio_id).ok_or_else(|| {
        ApiError::Invalid("未找到可用音频流（可能需要登录）".to_string())
    })?;
    let mut backup_urls = item.backup_url.unwrap_or_default();
    let url = match item.base_url {
        Some(base) => base,
        None => backup_urls
            .pop()
            .ok_or_else(|| ApiError::Invalid("音频流缺少 baseUrl 与 backupUrl".to_string()))?,
    };
    let codec = if item.codecs.contains("mp4a") {
        "mp4a".to_string()
    } else if item.codecs.contains("opus") {
        "opus".to_string()
    } else {
        item.codecs.clone()
    };
    Ok(AudioStream {
        url,
        backup_urls,
        audio_id: item.id,
        codec,
        bandwidth: item.bandwidth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_best_audio_filters_special_and_sorts() {
        let make = |id: u64, codec: &str, base: bool| DashAudioItem {
            id,
            base_url: if base { Some("https://cdn.example/a".to_string()) } else { None },
            backup_url: if base { None } else { Some(vec!["https://cdn.example/b".to_string()]) },
            codecs: codec.to_string(),
            bandwidth: 1000,
        };
        let items = vec![
            make(30216, "mp4a.40.2", true),
            make(30280, "mp4a.40.2", true),
            make(30250, "ec-3", true),
            make(30232, "mp4a.40.2", false),
            make(30251, "flac", true),
        ];
        let picked = pick_best_audio(items, None).expect("应选出普通 AAC");
        assert_eq!(picked.id, 30216, "可播放优先：默认选最低 AAC");
    }

    #[test]
    fn pick_best_audio_falls_back_to_opus_without_aac() {
        let make = |id: u64, codec: &str, base: bool| DashAudioItem {
            id,
            base_url: if base { Some("https://cdn.example/a".to_string()) } else { None },
            backup_url: if base { None } else { Some(vec!["https://cdn.example/b".to_string()]) },
            codecs: codec.to_string(),
            bandwidth: 1000,
        };
        let items = vec![
            make(30280, "opus", true),
            make(30232, "opus", true),
        ];
        let picked = pick_best_audio(items, None).expect("无 AAC 时应选 opus");
        assert_eq!(picked.id, 30232);
    }

    #[test]
    fn pick_best_audio_excludes_failed_id() {
        let make = |id: u64, codec: &str| DashAudioItem {
            id,
            base_url: Some("https://cdn.example/a".to_string()),
            backup_url: None,
            codecs: codec.to_string(),
            bandwidth: 1000,
        };
        let items = vec![
            make(30216, "mp4a.40.2"),
            make(30232, "mp4a.40.2"),
        ];
        let picked = pick_best_audio(items, Some(30216)).expect("应排除后选下一档");
        assert_eq!(picked.id, 30232);
    }

}
