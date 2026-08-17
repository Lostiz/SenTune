use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::cookie::Jar;
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::ApiError;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";
pub const REFERER: &str = "https://www.bilibili.com/";

const TIMEOUT_SECS: u64 = 15;
const RETRY_COUNT: usize = 1;

/// 手动维护的 Cookie 集合（请求头级别的确定性发送；
/// reqwest Jar 仍保留用于接收响应 Set-Cookie，登录阶段使用）。
static COOKIE_STATE: Mutex<Option<Vec<(String, String)>>> = Mutex::new(None);

pub mod device;
pub mod netease;
pub mod playurl;
pub mod search;
pub mod view;
pub mod wbi;

pub(crate) fn add_cookie(name: &str, value: &str) {
    let mut state = COOKIE_STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let entries = state.get_or_insert_with(Vec::new);
    if let Some(entry) = entries.iter_mut().find(|(key, _)| key == name) {
        entry.1 = value.to_string();
    } else {
        entries.push((name.to_string(), value.to_string()));
    }
}

fn cookie_header_value() -> Option<String> {
    let state = COOKIE_STATE.lock().ok()?;
    let entries = state.as_ref()?;
    if entries.is_empty() {
        return None;
    }
    Some(
        entries
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn cookie_jar() -> Arc<Jar> {
    static JAR: OnceLock<Arc<Jar>> = OnceLock::new();
    JAR.get_or_init(|| Arc::new(Jar::default())).clone()
}

pub(crate) fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static(REFERER),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .cookie_provider(cookie_jar())
            .build()
            .expect("reqwest 客户端初始化失败")
    })
}

pub(crate) fn blocking_client() -> reqwest::blocking::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static(REFERER),
    );
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("blocking 客户端初始化失败")
}

fn build_request(url: &str, params: &[(&str, &str)]) -> reqwest::RequestBuilder {
    let mut request = http_client().get(url);
    if !params.is_empty() {
        let pairs: Vec<(&str, &str)> = params.to_vec();
        request = request.query(&pairs);
    }
    if let Some(cookie) = cookie_header_value() {
        if let Ok(value) = header::HeaderValue::from_str(&cookie) {
            request = request.header(header::COOKIE, value);
        }
    }
    request
}

/// 统一 GET 入口：UA / Referer / CookieStore / 15s 超时 / 失败重试 1 次。
pub async fn get_json(url: &str, params: &[(&str, &str)]) -> Result<Value, ApiError> {
    let mut last_error: Option<ApiError> = None;
    for attempt in 0..=RETRY_COUNT {
        match get_json_once(url, params).await {
            Ok(value) => return Ok(value),
            Err(error) => {
                last_error = Some(error);
                if attempt < RETRY_COUNT {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
    }
    Err(last_error.expect("重试循环后必然存在错误"))
}

async fn get_json_once(url: &str, params: &[(&str, &str)]) -> Result<Value, ApiError> {
    device::ensure_device_fingerprint();
    let response = build_request(url, params).send().await?;
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

/// 解析 B 站统一响应包裹（code/message/data），code != 0 时返回结构化错误。
pub async fn get_data<T: DeserializeOwned>(
    url: &str,
    params: &[(&str, &str)],
) -> Result<T, ApiError> {
    let value = get_json(url, params).await?;
    let code = value.get("code").and_then(Value::as_i64).unwrap_or(-1);
    if code != 0 {
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("未知错误")
            .to_string();
        if code == -101 {
            return Err(ApiError::Unauthorized);
        }
        return Err(ApiError::Bili { code, message });
    }
    let data = value
        .get("data")
        .ok_or_else(|| ApiError::Bili {
            code,
            message: "响应缺少 data 字段".to_string(),
        })?;
    if data.is_null() {
        return Err(ApiError::Bili {
            code,
            message: "响应 data 为空".to_string(),
        });
    }
    let parsed: T = serde_json::from_value(data.clone())?;
    Ok(parsed)
}
