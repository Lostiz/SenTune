use std::sync::{Mutex, Once};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use reqwest::Url;
use sha2::Sha256;

use super::{http_client, ApiError};

type HmacSha256 = Hmac<Sha256>;

const TICKET_HMAC_KEY: &[u8] = b"XgwSnGZ1p";
const TICKET_URL: &str =
    "https://api.bilibili.com/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket";
/// 票据实际有效约 3 天，提前到 12 小时刷新，失败时保持旧票据并使用重试逻辑。
const TICKET_REFRESH_SECS: u64 = 12 * 60 * 60;

static DEVICE_ONCE: Once = Once::new();
static TICKET_CACHE: Mutex<Option<(String, u64)>> = Mutex::new(None);
static TICKET_FETCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 生成 RFC 4122 v4 UUID（大写、无连字符，32 位十六进制）。
fn random_uuid_hex() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn random_buvid4() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// `b_lsid`：8 位大写十六进制 `_` 8 位大写十六进制。
fn random_b_lsid() -> String {
    let mut lsid = String::with_capacity(17);
    for _ in 0..2 {
        let mut bytes = [0u8; 4];
        rand::rng().fill_bytes(&mut bytes);
        lsid.push_str(&bytes.iter().map(|byte| format!("{byte:02X}")).collect::<String>());
        if lsid.len() == 8 {
            lsid.push('_');
        }
    }
    lsid
}

fn set_cookie(name: &str, value: &str) {
    let cookie = format!("{name}={value}; Domain=.bilibili.com; Path=/");
    let url = Url::parse("https://api.bilibili.com/").expect("常量 URL 必然合法");
    super::cookie_jar().add_cookie_str(&cookie, &url);
    super::add_cookie(name, value);
}

/// 一次性生成并写入设备指纹 Cookie（buvid3 / buvid4 / b_nut / _uuid / b_lsid）。
pub fn ensure_device_fingerprint() {
    DEVICE_ONCE.call_once(|| {
        set_cookie("buvid3", &format!("{}_infoc", random_uuid_hex()));
        set_cookie("buvid4", &random_buvid4());
        set_cookie("b_nut", &now_millis().to_string());
        set_cookie("_uuid", &format!("{}_infoc", random_uuid_hex()));
        set_cookie("b_lsid", &random_b_lsid());
    });
}

/// GenWebTicket 的 hexsign：HMAC-SHA256(密钥, "ts" + 秒级时间戳) 的十六进制小写。
fn hex_sign(ts: u64) -> String {
    let mut mac = HmacSha256::new_from_slice(TICKET_HMAC_KEY).expect("HMAC 接受任意长度密钥");
    mac.update(format!("ts{ts}").as_bytes());
    let digest = mac.finalize().into_bytes();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn fetch_ticket() -> Result<String, ApiError> {
    let ts = now_secs();
    let hexsign = hex_sign(ts);
    let url = format!(
        "{TICKET_URL}?key_id=ec02&hexsign={hexsign}&context%5Bts%5D={ts}&csrf="
    );
    let response = http_client().post(url).send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ApiError::Http {
            status: status.as_u16(),
            body: body.chars().take(200).collect(),
        });
    }
    let value: serde_json::Value = serde_json::from_str(&body)?;
    let code = value.get("code").and_then(serde_json::Value::as_i64).unwrap_or(-1);
    if code != 0 {
        let message = value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("未知错误")
            .to_string();
        return Err(ApiError::Bili { code, message });
    }
    let ticket = value
        .pointer("/data/ticket")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ApiError::Invalid("GenWebTicket 响应缺少 data.ticket".to_string()))?;
    if ticket.is_empty() {
        return Err(ApiError::Invalid("GenWebTicket 返回空票据".to_string()));
    }
    set_cookie("bili_ticket", ticket);
    Ok(ticket.to_string())
}

/// 确保内存缓存中有一张未过期的 bili_ticket，并已写入共享 Cookie Jar。
pub async fn ensure_bili_ticket() -> Result<(), ApiError> {
    ensure_device_fingerprint();
    let now = now_secs();
    if let Ok(guard) = TICKET_CACHE.lock() {
        if let Some((_, fetched_at)) = guard.as_ref() {
            if now.saturating_sub(*fetched_at) < TICKET_REFRESH_SECS {
                return Ok(());
            }
        }
    }
    // 串行化首次获取，避免并发测试/调用同时打 GenWebTicket 触发风控。
    let _guard = TICKET_FETCH_LOCK.lock().await;
    if let Ok(guard) = TICKET_CACHE.lock() {
        if let Some((_, fetched_at)) = guard.as_ref() {
            if now.saturating_sub(*fetched_at) < TICKET_REFRESH_SECS {
                return Ok(());
            }
        }
    }
    let ticket = fetch_ticket().await?;
    if let Ok(mut guard) = TICKET_CACHE.lock() {
        *guard = Some((ticket, now));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_sign_matches_known_vector() {
        // 由 Node crypto.createHmac("sha256", "XgwSnGZ1p").update("ts1752000000") 独立计算。
        assert_eq!(
            hex_sign(1_752_000_000),
            "489caff403f910640bea7a3d75a668922ea33fb0facdb22088141a1aa90e1b5d"
        );
    }

    #[test]
    fn fingerprint_generators_have_expected_shape() {
        let uuid = random_uuid_hex();
        assert_eq!(uuid.len(), 32);
        assert!(uuid.chars().all(|c| c.is_ascii_hexdigit()));

        let buvid4 = random_buvid4();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&buvid4)
            .expect("buvid4 应为合法 base64");
        assert_eq!(decoded.len(), 32);

        let lsid = random_b_lsid();
        assert_eq!(lsid.len(), 17);
        let parts: Vec<&str> = lsid.split('_').collect();
        assert_eq!(parts.len(), 2, "b_lsid 应包含一个下划线");
        for part in parts {
            assert_eq!(part.len(), 8);
            assert!(part.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[tokio::test]
    #[ignore = "需要真实网络访问 B 站接口"]
    async fn live_bili_ticket_obtains_ticket() {
        ensure_bili_ticket().await.expect("获取 bili_ticket 应成功");
        let cached = TICKET_CACHE
            .lock()
            .expect("缓存锁可用")
            .as_ref()
            .map(|(ticket, _)| ticket.clone())
            .expect("票据应已缓存");
        assert_eq!(cached.split('.').count(), 3, "bili_ticket 应为 JWT 格式");
        assert!(!cached.is_empty());
    }
}
