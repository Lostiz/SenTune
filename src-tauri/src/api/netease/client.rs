//! 网易云公开 API HTTP 客户端：统一请求头、Cookie 管理、限流、失败退让与熔断。
//!
//! weapi 加密接口已失效（返回空响应），改用无需加密的公开接口
//! （`/api/cloudsearch/pc` 搜索、`/api/song/enhance/player/url` 播放地址）。
//!
//! 抗风控最低成本策略（对公开接口同样适用）：
//! - 身份稳定：固定 UA / Referer / Origin，Cookie（NMTID 等）持久化到 settings 表，
//!   重启不换指纹；
//! - 请求节流：搜索串行（1 并发 + ≥800ms 间隔），playurl 并发 ≤3；
//! - 失败退让：网络错误重试 2 次；业务码 -460 指数退避重试，连续触发熔断 60 秒；
//! - 熔断期间已有缓存/本地曲目不受影响（不经过本模块）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header;
use serde_json::Value;

use crate::logging;
use crate::models::ApiError;

const HOST: &str = "https://music.163.com";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const REFERER: &str = "https://music.163.com/";

/// 网络错误最大重试次数（含首次共 3 次）。
const NETWORK_ATTEMPTS: usize = 3;
/// -460 风控码指数退避重试次数。
const RISK_ATTEMPTS: usize = 3;
/// 连续 -460 达到该次数后打开熔断。
const CIRCUIT_THRESHOLD: u64 = 3;
/// 熔断时长。
const CIRCUIT_OPEN_SECS: u64 = 60;
/// 搜索最小间隔。
const SEARCH_MIN_INTERVAL_MS: u64 = 800;
/// playurl 最大并发。
const PLAYURL_MAX_CONCURRENCY: usize = 3;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// 请求类别：搜索、播放地址与歌词分开限流与熔断计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateKind {
    Search,
    Playurl,
    Lyric,
}

struct Breaker {
    consecutive: &'static AtomicU64,
    open_until: &'static AtomicU64,
}

impl Breaker {
    fn is_open(&self) -> bool {
        self.open_until.load(Ordering::SeqCst) > now_ms()
    }

    fn record_success(&self) {
        self.consecutive.store(0, Ordering::SeqCst);
    }

    /// 记录一次 -460；达到阈值时打开熔断，返回熔断是否因此打开。
    fn record_risk(&self) -> bool {
        let count = self.consecutive.fetch_add(1, Ordering::SeqCst) + 1;
        if count >= CIRCUIT_THRESHOLD {
            self.open_until
                .store(now_ms() + CIRCUIT_OPEN_SECS * 1000, Ordering::SeqCst);
            self.consecutive.store(0, Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}

impl RateKind {
    fn breaker(self) -> Breaker {
        static SEARCH_CONSECUTIVE: AtomicU64 = AtomicU64::new(0);
        static SEARCH_OPEN_UNTIL: AtomicU64 = AtomicU64::new(0);
        static PLAYURL_CONSECUTIVE: AtomicU64 = AtomicU64::new(0);
        static PLAYURL_OPEN_UNTIL: AtomicU64 = AtomicU64::new(0);
        static LYRIC_CONSECUTIVE: AtomicU64 = AtomicU64::new(0);
        static LYRIC_OPEN_UNTIL: AtomicU64 = AtomicU64::new(0);
        match self {
            RateKind::Search => Breaker {
                consecutive: &SEARCH_CONSECUTIVE,
                open_until: &SEARCH_OPEN_UNTIL,
            },
            RateKind::Playurl => Breaker {
                consecutive: &PLAYURL_CONSECUTIVE,
                open_until: &PLAYURL_OPEN_UNTIL,
            },
            RateKind::Lyric => Breaker {
                consecutive: &LYRIC_CONSECUTIVE,
                open_until: &LYRIC_OPEN_UNTIL,
            },
        }
    }
}

/// 限流许可：在请求全程持有，函数返回时释放（字段仅用于 RAII，不读取）。
#[allow(dead_code)]
enum RatePermit {
    Search(tokio::sync::MutexGuard<'static, ()>),
    Playurl(tokio::sync::SemaphorePermit<'static>),
}

async fn acquire_slot(kind: RateKind) -> RatePermit {
    match kind {
        RateKind::Search => {
            static GATE: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
            static LAST_SEARCH_MS: AtomicU64 = AtomicU64::new(0);
            let gate = GATE.get_or_init(|| tokio::sync::Mutex::new(()));
            let guard = gate.lock().await;
            let last = LAST_SEARCH_MS.load(Ordering::SeqCst);
            let elapsed = now_ms().saturating_sub(last);
            if elapsed < SEARCH_MIN_INTERVAL_MS {
                tokio::time::sleep(Duration::from_millis(SEARCH_MIN_INTERVAL_MS - elapsed))
                    .await;
            }
            LAST_SEARCH_MS.store(now_ms(), Ordering::SeqCst);
            RatePermit::Search(guard)
        }
        RateKind::Playurl | RateKind::Lyric => {
            static SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
            let semaphore =
                SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(PLAYURL_MAX_CONCURRENCY));
            let permit = semaphore.acquire().await.expect("信号量不应关闭");
            RatePermit::Playurl(permit)
        }
    }
}

/// 手动维护的 Cookie 串（key=value; ...）。
static COOKIE: Mutex<Option<String>> = Mutex::new(None);
/// 防止并发时重复做 Cookie 预热。
static COOKIE_READY: AtomicU64 = AtomicU64::new(0);

const COOKIE_READY_PENDING: u64 = 0;
const COOKIE_READY_DONE: u64 = 1;
const COOKIE_SETTINGS_KEY: &str = "netease_cookie";

fn cookie_header() -> Option<String> {
    COOKIE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn set_cookie(value: String) {
    // 先持久化（借用 value），再写入内存状态。
    if let Some(connection) = crate::db::try_connection() {
        let guard = connection.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = crate::db::settings::set(&guard, COOKIE_SETTINGS_KEY, &value);
    }
    *COOKIE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(value);
}

/// 合并 Set-Cookie 响应头为 cookie 串（只保留 name=value 部分）。
fn merge_set_cookies(existing: Option<&str>, headers: &header::HeaderMap) -> String {
    let mut map: Vec<(String, String)> = Vec::new();
    if let Some(existing) = existing {
        for pair in existing.split(';') {
            let pair = pair.trim();
            if let Some((name, value)) = pair.split_once('=') {
                map.push((name.trim().to_string(), value.trim().to_string()));
            }
        }
    }
    for value in headers.get_all(header::SET_COOKIE) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        let Some(pair) = value.split(';').next() else {
            continue;
        };
        let Some((name, cookie_value)) = pair.split_once('=') else {
            continue;
        };
        if let Some(entry) = map.iter_mut().find(|(key, _)| key == name.trim()) {
            entry.1 = cookie_value.trim().to_string();
        } else {
            map.push((name.trim().to_string(), cookie_value.trim().to_string()));
        }
    }
    map.into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// 生成稳定的随机 NMTID（32 位十六进制）：网易云不强制服务端下发 Cookie，
/// 客户端自造 NMTID 并保持稳定即可（与 NeteaseCloudMusicApi 同做法）。
fn random_nmtid() -> String {
    (0..32)
        .map(|_| format!("{:x}", rand::random::<u8>() % 16))
        .collect()
}

/// 预热 Cookie：先读持久化值，再访问首页刷新；失败不阻断后续请求。
async fn ensure_cookie() {
    // 预热只做一次（首个请求完成后其余请求直接用）。
    if COOKIE_READY.compare_exchange(
        COOKIE_READY_PENDING,
        COOKIE_READY_DONE,
        Ordering::SeqCst,
        Ordering::SeqCst,
    )
    .is_err()
    {
        return;
    }
    let persisted = crate::db::try_connection().and_then(|connection| {
        let guard = connection.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        crate::db::settings::get(&guard, COOKIE_SETTINGS_KEY).ok().flatten()
    });
    // 无持久化 Cookie 时自造稳定 NMTID（重启前保持不变，重启后从 settings 恢复）。
    let initial = persisted.unwrap_or_else(|| {
        format!("os=pc; appver=8.9.70; NMTID={}", random_nmtid())
    });
    set_cookie(initial);
    match netease_client().get(HOST).send().await {
        Ok(response) => {
            let merged = merge_set_cookies(cookie_header().as_deref(), response.headers());
            if !merged.is_empty() {
                set_cookie(merged);
            }
        }
        Err(error) => {
            logging::info(&format!("网易云 Cookie 预热失败（忽略）：{error}"));
        }
    }
}

pub(crate) fn netease_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static(REFERER),
        );
        headers.insert(
            header::ORIGIN,
            header::HeaderValue::from_static(HOST),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()
            .expect("网易云 reqwest 客户端初始化失败")
    })
}

/// 网易云 CDN 下载用阻塞客户端（UA/Referer + Cookie），供下载线程使用。
pub(crate) fn netease_blocking_client() -> reqwest::blocking::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static(REFERER),
    );
    if let Some(cookie) = cookie_header() {
        if let Ok(value) = header::HeaderValue::from_str(&cookie) {
            headers.insert(header::COOKIE, value);
        }
    }
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("网易云 blocking 客户端初始化失败")
}

/// 表单百分号编码（值中非保留字符转义）。
fn form_encode(key: &str, value: &str) -> String {
    let mut output = String::with_capacity(key.len() + value.len() + 1);
    output.push_str(key);
    output.push('=');
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

async fn send_once(request: reqwest::RequestBuilder) -> Result<Value, ApiError> {
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ApiError::Http {
            status: status.as_u16(),
            body: body.chars().take(200).collect(),
        });
    }
    let value: Value = serde_json::from_str(&body)?;
    Ok(value)
}

fn risk_message() -> String {
    "网易云请求过于频繁，请稍后再试".to_string()
}

fn risk_error() -> ApiError {
    ApiError::Netease {
        code: -460,
        message: risk_message(),
    }
}

/// 统一请求入口：Cookie 预热、限流、业务码校验、退避重试、熔断。
async fn send_with_policy(
    kind: RateKind,
    request: reqwest::RequestBuilder,
) -> Result<Value, ApiError> {
    ensure_cookie().await;
    let _permit = acquire_slot(kind).await;
    let breaker = kind.breaker();
    if breaker.is_open() {
        return Err(risk_error());
    }

    let mut last_network_error: Option<ApiError> = None;
    for attempt in 0..NETWORK_ATTEMPTS {
        if breaker.is_open() {
            return Err(risk_error());
        }
        let Some(cloned) = request.try_clone() else {
            return Err(ApiError::Invalid("请求体无法复制，无法重试".to_string()));
        };
        match send_once(cloned).await {
            Ok(value) => {
                let code = value.get("code").and_then(Value::as_i64).unwrap_or(0);
                if code == 200 {
                    breaker.record_success();
                    return Ok(value);
                }
                let message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("未知错误")
                    .to_string();
                if code == -460 {
                    let opened = breaker.record_risk();
                    logging::info(&format!(
                        "网易云风控 -460（{}），重试 {attempt}/{}{}",
                        if opened { "已熔断" } else { "未熔断" },
                        RISK_ATTEMPTS - 1,
                        if opened { "，熔断 60 秒" } else { "" },
                    ));
                    if attempt + 1 < NETWORK_ATTEMPTS && attempt + 1 < RISK_ATTEMPTS {
                        // 指数退避 1s / 2s + 抖动。
                        let backoff_ms = (1u64 << attempt) * 1000 + rand::random::<u64>() % 300;
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    return Err(risk_error());
                }
                return Err(ApiError::Netease { code, message });
            }
            Err(error) => {
                last_network_error = Some(error);
                if attempt + 1 < NETWORK_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                }
            }
        }
    }
    Err(last_network_error.expect("重试循环后必然存在错误"))
}

/// 公开 API GET（带 Cookie）。
pub async fn api_get(
    path: &str,
    params: &[(&str, &str)],
    kind: RateKind,
) -> Result<Value, ApiError> {
    let mut request = netease_client().get(format!("{HOST}{path}"));
    if !params.is_empty() {
        let pairs: Vec<(&str, &str)> = params.to_vec();
        request = request.query(&pairs);
    }
    if let Some(cookie) = cookie_header() {
        if let Ok(value) = header::HeaderValue::from_str(&cookie) {
            request = request.header(header::COOKIE, value);
        }
    }
    send_with_policy(kind, request).await
}

/// 公开 API POST 表单（带 Cookie）。
pub async fn api_post_form(
    path: &str,
    form: &[(&str, &str)],
    kind: RateKind,
) -> Result<Value, ApiError> {
    let body = form
        .iter()
        .map(|(key, value)| form_encode(key, value))
        .collect::<Vec<_>>()
        .join("&");
    let mut request = netease_client()
        .post(format!("{HOST}{path}"))
        .header(
            header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body);
    if let Some(cookie) = cookie_header() {
        if let Ok(value) = header::HeaderValue::from_str(&cookie) {
            request = request.header(header::COOKIE, value);
        }
    }
    send_with_policy(kind, request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_encoding_handles_special_symbols() {
        assert_eq!(form_encode("ids", "[123]"), "ids=%5B123%5D");
        assert_eq!(form_encode("br", "128000"), "br=128000");
    }

    #[test]
    fn merge_set_cookies_keeps_latest_value() {
        let mut headers = header::HeaderMap::new();
        headers.append(
            header::SET_COOKIE,
            header::HeaderValue::from_static("NMTID=abc123; Path=/; HttpOnly"),
        );
        headers.append(
            header::SET_COOKIE,
            header::HeaderValue::from_static("_iuqxldmzr_=25; Path=/"),
        );
        headers.append(
            header::SET_COOKIE,
            header::HeaderValue::from_static("NMTID=new456; Path=/"),
        );
        let merged = merge_set_cookies(None, &headers);
        assert!(merged.contains("NMTID=new456"), "同名 Cookie 应取最新值：{merged}");
        assert!(merged.contains("_iuqxldmzr_=25"));
        assert!(!merged.contains("abc123"), "旧值应被覆盖");

        // 与已有 Cookie 合并。
        let merged_with_existing =
            merge_set_cookies(Some("os=pc; appver=8.9.70"), &header::HeaderMap::new());
        assert!(merged_with_existing.contains("os=pc"));
        assert!(merged_with_existing.contains("appver=8.9.70"));
    }

    #[test]
    fn nmtid_is_32_hex_chars() {
        let value = random_nmtid();
        assert_eq!(value.len(), 32);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
