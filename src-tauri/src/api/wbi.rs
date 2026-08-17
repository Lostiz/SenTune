use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};

use super::{get_json, ApiError};

/// 固定 64 位打乱表（官方算法）。
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19,
    29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

/// nav 接口不可用时的内置 fallback key（2023-09 起长期有效的公开默认值）。
const FALLBACK_IMG_KEY: &str = "7cd084941338484aae1ad9425b84077c";
const FALLBACK_SUB_KEY: &str = "4932caff0ff746eab6f01bf08b70ac45";

const KEY_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

struct WbiKeyCache {
    img_key: String,
    sub_key: String,
    fetched_at: u64,
}

static WBI_KEY_CACHE: Mutex<Option<WbiKeyCache>> = Mutex::new(None);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn file_key_from_url(url: &str) -> Option<String> {
    let name = url.rsplit('/').next()?;
    let stem = name.split('.').next().unwrap_or(name);
    if stem.is_empty() {
        None
    } else {
        Some(stem.to_string())
    }
}

async fn fetch_keys_remote() -> Result<(String, String), ApiError> {
    let value = get_json("https://api.bilibili.com/x/web-interface/nav", &[]).await?;
    let img_url = value
        .pointer("/data/wbi_img/img_url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ApiError::Invalid("nav 响应缺少 wbi_img.img_url".to_string()))?;
    let sub_url = value
        .pointer("/data/wbi_img/sub_url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ApiError::Invalid("nav 响应缺少 wbi_img.sub_url".to_string()))?;
    let img_key = file_key_from_url(img_url)
        .ok_or_else(|| ApiError::Invalid("无法解析 img_key".to_string()))?;
    let sub_key = file_key_from_url(sub_url)
        .ok_or_else(|| ApiError::Invalid("无法解析 sub_key".to_string()))?;
    Ok((img_key, sub_key))
}

/// 获取 img_key/sub_key：优先使用缓存，其次实时拉取，失败时回退内置 key。
async fn get_wbi_keys() -> (String, String) {
    let now = now_secs();
    if let Ok(guard) = WBI_KEY_CACHE.lock() {
        if let Some(cache) = guard.as_ref() {
            if now.saturating_sub(cache.fetched_at) < KEY_CACHE_TTL_SECS {
                return (cache.img_key.clone(), cache.sub_key.clone());
            }
        }
    }

    match fetch_keys_remote().await {
        Ok((img_key, sub_key)) => {
            if let Ok(mut guard) = WBI_KEY_CACHE.lock() {
                *guard = Some(WbiKeyCache {
                    img_key: img_key.clone(),
                    sub_key: sub_key.clone(),
                    fetched_at: now,
                });
            }
            (img_key, sub_key)
        }
        Err(_) => (FALLBACK_IMG_KEY.to_string(), FALLBACK_SUB_KEY.to_string()),
    }
}

fn percent_encode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn hmac_md5(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut key = key.to_vec();
    if key.len() > 64 {
        key = Md5::digest(&key).to_vec();
    }
    key.resize(64, 0);

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (index, byte) in key.iter().enumerate() {
        ipad[index] ^= byte;
        opad[index] ^= byte;
    }

    let mut inner = Md5::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Md5::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().to_vec()
}

fn hmac_md5_hex(key: &[u8], message: &[u8]) -> String {
    hmac_md5(key, message)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// 按 7.2 构造签名查询串：过滤 w_rid/wts → 按键排序 → URL 编码 → 末尾追加 wts。
fn build_signed_query(params: &[(&str, &str)], wts: u64) -> String {
    let mut sorted: BTreeMap<String, String> = BTreeMap::new();
    for (key, value) in params {
        if *key == "w_rid" || *key == "wts" {
            continue;
        }
        sorted.insert(key.to_string(), value.to_string());
    }
    let mut pairs: Vec<String> = sorted
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect();
    pairs.push(format!("wts={wts}"));
    pairs.join("&")
}

/// 为参数追加 wts 与 w_rid（wbi 签名）。
pub async fn sign_params(params: &mut Vec<(String, String)>) -> Result<(), ApiError> {
    let (img_key, sub_key) = get_wbi_keys().await;
    let mixin_key = format!("{img_key}{sub_key}");
    let wts = now_secs();
    let refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let query = build_signed_query(&refs, wts);
    let w_rid = hmac_md5_hex(mixin_key.as_bytes(), query.as_bytes());
    params.retain(|(key, _)| key != "w_rid");
    params.push(("wts".to_string(), wts.to_string()));
    params.push(("w_rid".to_string(), w_rid));
    Ok(())
}

/// 打乱表仅保留为与官方实现一致的常量（未来算法变更时使用）。
#[allow(dead_code)]
fn _mixin_key_enc_tab() -> &'static [usize; 64] {
    &MIXIN_KEY_ENC_TAB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_keeps_unreserved_and_encodes_chinese() {
        assert_eq!(percent_encode("abc-_.~"), "abc-_.~");
        assert_eq!(percent_encode("纯音乐"), "%E7%BA%AF%E9%9F%B3%E4%B9%90");
    }

    #[test]
    fn signed_query_matches_known_vector() {
        // 固定输入 + 固定 key + 固定 wts，产出固定的 w_rid（真实可复现）。
        let img_key = "7cd084941338484aae1ad9425b84077c";
        let sub_key = "4932caff0ff746eab6f01bf08b70ac45";
        let mixin_key = format!("{img_key}{sub_key}");
        let params = [
            ("keyword", "纯音乐"),
            ("page", "1"),
            ("page_size", "20"),
            ("search_type", "video"),
        ];
        let wts = 1_752_000_000u64;
        let query = build_signed_query(&params, wts);
        assert_eq!(
            query,
            "keyword=%E7%BA%AF%E9%9F%B3%E4%B9%90&page=1&page_size=20&search_type=video&wts=1752000000"
        );
        let w_rid = hmac_md5_hex(mixin_key.as_bytes(), query.as_bytes());
        // 由 Node crypto.createHmac("md5", mixin).update(query).digest("hex") 独立计算所得。
        assert_eq!(
            w_rid,
            "36b77948c033d67658a881da4e05199a",
            "wbi 签名结果与已知向量不一致"
        );
    }
}
