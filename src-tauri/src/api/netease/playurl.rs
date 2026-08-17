//! 网易云播放地址：公开接口 `POST /api/song/enhance/player/url`。
//!
//! 无登录态只能取 `br=128000`（128kbps）；URL 有时效，每次播放现取，不落库复用。
//! CDN 返回的地址为 http，统一改写为 https（m*.music.126.net 支持）。

use serde_json::Value;

use super::client::{api_post_form, RateKind};
use crate::models::{ApiError, NeteaseStream};

const PLAYURL_PATH: &str = "/api/song/enhance/player/url";

/// 将 CDN 地址的 http 方案改写为 https。
fn to_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{rest}")
    } else {
        url.to_string()
    }
}

pub async fn get_play_url(song_id: u64) -> Result<NeteaseStream, ApiError> {
    if song_id == 0 {
        return Err(ApiError::Invalid("歌曲 ID 不能为空".to_string()));
    }
    let raw_form = vec![
        ("ids".to_string(), format!("[{song_id}]")),
        ("br".to_string(), "128000".to_string()),
    ];
    let form: Vec<(&str, &str)> = raw_form
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let value = api_post_form(PLAYURL_PATH, &form, RateKind::Playurl).await?;
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::Netease {
            code: 0,
            message: "响应缺少 data 字段".to_string(),
        })?;
    let first = data.first().ok_or_else(|| ApiError::Netease {
        code: 0,
        message: "未返回播放地址".to_string(),
    })?;
    let code = first.get("code").and_then(Value::as_i64).unwrap_or(404);
    let url = first.get("url").and_then(Value::as_str).unwrap_or("");
    if url.is_empty() {
        // -110 = VIP/付费；404 = 无版权；其余不可播同样没有可用地址。
        return Err(ApiError::Netease {
            code,
            message: "该歌曲需要会员或不可播放".to_string(),
        });
    }
    let codec = first
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("mp3")
        .to_string();
    Ok(NeteaseStream {
        url: to_https(url),
        bitrate: first.get("br").and_then(Value::as_u64).unwrap_or(128_000),
        codec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_scheme_rewritten_to_https() {
        assert_eq!(
            to_https("http://m801.music.126.net/a.mp3"),
            "https://m801.music.126.net/a.mp3"
        );
        assert_eq!(
            to_https("https://m801.music.126.net/a.mp3"),
            "https://m801.music.126.net/a.mp3"
        );
    }

    #[test]
    fn empty_song_id_rejected() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("运行时构建失败");
        let error = runtime
            .block_on(get_play_url(0))
            .expect_err("song_id=0 应报错");
        assert!(error.to_string().contains("歌曲 ID"));
    }

    #[tokio::test]
    #[ignore = "需要真实网络访问网易云 CDN"]
    async fn live_netease_stream_downloads_and_caches() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-netease-live-{}",
            std::process::id()
        ));
        let cache_root = dir.join("cache");
        std::fs::create_dir_all(&cache_root).expect("临时目录应可创建");
        crate::db::init(&dir.join("sentune.db")).expect("数据库应可初始化");

        // 搜索一首免费歌曲并解析播放地址。
        let page = crate::api::netease::search("纯音乐", 1)
            .await
            .expect("搜索应成功");
        let free = page
            .items
            .iter()
            .find(|song| song.fee == 0)
            .expect("结果中应有免费歌曲");
        let stream = crate::api::netease::get_play_url(free.id)
            .await
            .expect("播放地址应可解析");

        // 走边下边播管线：后台下载器应完整下载并原子落盘。
        let task = crate::stream::start_netease_stream_task(free, &stream, &cache_root)
            .expect("任务应可创建");
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !task.cache_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(task.cache_path.exists(), "完成后应生成正式缓存文件");
        let size = std::fs::metadata(&task.cache_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        assert!(size > 100_000, "缓存文件应有实际音频内容（{size} 字节）");
        assert_eq!(
            crate::stream::stream_status(&task, 0).status,
            "completed"
        );

        // 再次播放同一曲目：应命中本地完成态，不联网、不重新下载。
        // （完成落盘与 DB 标记存在微小时差，轮询等待标记完成。）
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut cached = None;
        while std::time::Instant::now() < deadline {
            if let Ok(Some(task)) = crate::stream::try_start_cached_netease(free.id) {
                cached = Some(task);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let cached = cached.expect("已缓存曲目应返回本地任务");
        assert_eq!(
            cached.status.load(std::sync::atomic::Ordering::SeqCst),
            crate::stream::STATUS_COMPLETED
        );
        assert!(cached.audio_urls.is_empty(), "本地播放不应携带 CDN 地址");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
