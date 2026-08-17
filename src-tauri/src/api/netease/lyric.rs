//! 网易云歌词：公开接口 `GET /api/song/lyric`。
//!
//! 使用无登录公开接口获取原文歌词（lrc）与翻译歌词（tlyric），
//! 复用统一 UA / Referer / Cookie / 限流 / 熔断策略。

use serde_json::Value;

use super::client::{api_get, RateKind};
use crate::models::{ApiError, NeteaseLyric};

const LYRIC_PATH: &str = "/api/song/lyric";

/// 从响应中提取指定 key（如 `lrc` / `tlyric`）下的 `lyric` 文本。
fn extract_lyric(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|section| section.get("lyric"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

pub async fn get_lyric(song_id: u64) -> Result<NeteaseLyric, ApiError> {
    if song_id == 0 {
        return Err(ApiError::Invalid("歌曲 ID 不能为空".to_string()));
    }

    let id = song_id.to_string();
    let params = vec![
        ("id", id.as_str()),
        ("lv", "-1"),
        ("kv", "-1"),
        ("tv", "-1"),
    ];
    let value = api_get(LYRIC_PATH, &params, RateKind::Lyric).await?;

    Ok(NeteaseLyric {
        song_id,
        lyric: extract_lyric(&value, "lrc"),
        translated_lyric: extract_lyric(&value, "tlyric"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_lyric_sections() {
        let json = serde_json::json!({
            "code": 200,
            "lrc": {
                "version": 7,
                "lyric": "[00:00.00] 测试歌词"
            },
            "tlyric": {
                "version": 1,
                "lyric": "[00:00.00] Test Lyric"
            }
        });

        assert_eq!(
            extract_lyric(&json, "lrc").as_deref(),
            Some("[00:00.00] 测试歌词")
        );
        assert_eq!(
            extract_lyric(&json, "tlyric").as_deref(),
            Some("[00:00.00] Test Lyric")
        );
        assert_eq!(extract_lyric(&json, "klyric"), None);
    }

    #[test]
    fn empty_lyric_becomes_none() {
        let json = serde_json::json!({
            "code": 200,
            "lrc": { "lyric": "" },
            "tlyric": { "lyric": "   " }
        });
        assert_eq!(extract_lyric(&json, "lrc"), None);
        assert_eq!(extract_lyric(&json, "tlyric"), None);
    }
}
