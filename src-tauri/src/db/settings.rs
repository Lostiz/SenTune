use rusqlite::Connection;
use serde::Serialize;

use crate::models::ApiError;

pub const DEFAULT_KEEP_DAYS: i64 = 7;
pub const MIN_CAPACITY_GB: i64 = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSettings {
    pub cache_path: Option<String>,
    pub keep_days: i64,
    pub capacity_limit_gb: i64,
}

pub fn get(connection: &Connection, key: &str) -> Result<Option<String>, ApiError> {
    use rusqlite::OptionalExtension;
    Ok(connection
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn set(connection: &Connection, key: &str, value: &str) -> Result<(), ApiError> {
    connection.execute(
        r#"
        INSERT INTO settings (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn load_cache_settings(connection: &Connection) -> Result<CacheSettings, ApiError> {
    let keep_days = get(connection, "keep_days")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_KEEP_DAYS)
        .max(1);
    let capacity_limit_gb = get(connection, "capacity_limit_gb")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let capacity_limit_gb = if capacity_limit_gb == 0 {
        0
    } else {
        capacity_limit_gb.max(MIN_CAPACITY_GB)
    };
    Ok(CacheSettings {
        cache_path: get(connection, "cache_path")?,
        keep_days,
        capacity_limit_gb,
    })
}

pub fn save_cache_settings(
    connection: &Connection,
    settings: &CacheSettings,
) -> Result<(), ApiError> {
    if let Some(path) = &settings.cache_path {
        set(connection, "cache_path", path)?;
    }
    set(connection, "keep_days", &settings.keep_days.to_string())?;
    set(
        connection,
        "capacity_limit_gb",
        &settings.capacity_limit_gb.to_string(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_with_defaults() {
        let connection =
            rusqlite::Connection::open_in_memory().expect("内存数据库应可打开");
        connection
            .execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .expect("建表应成功");
        let defaults = load_cache_settings(&connection).expect("默认值应可读");
        assert_eq!(defaults.keep_days, 7);
        assert_eq!(defaults.capacity_limit_gb, 0);
        assert!(defaults.cache_path.is_none());

        let saved = CacheSettings {
            cache_path: Some("D:\\cache".to_string()),
            keep_days: 3,
            capacity_limit_gb: 5,
        };
        save_cache_settings(&connection, &saved).expect("保存应成功");
        let loaded = load_cache_settings(&connection).expect("读取应成功");
        assert_eq!(loaded.keep_days, 3);
        assert_eq!(loaded.capacity_limit_gb, 5);
        assert_eq!(loaded.cache_path.as_deref(), Some("D:\\cache"));

        // 低于下限的容量值会被抬升到 5GB。
        let invalid = CacheSettings {
            cache_path: None,
            keep_days: 1,
            capacity_limit_gb: 2,
        };
        save_cache_settings(&connection, &invalid).expect("保存应成功");
        let loaded = load_cache_settings(&connection).expect("读取应成功");
        assert_eq!(loaded.capacity_limit_gb, 5);
    }
}
