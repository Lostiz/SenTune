use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::models::ApiError;

#[derive(Debug, Clone)]
pub struct TrackRecord {
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub cover_url: String,
    pub author: String,
    pub duration: u64,
    pub audio_id: u64,
    pub codec: String,
    pub cache_path: Option<String>,
    pub cached_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub cover_url: String,
    pub author: String,
    pub duration: u64,
    pub audio_id: u64,
    pub codec: String,
    pub cache_path: Option<String>,
    pub cached_at: Option<i64>,
    pub last_played_at: Option<i64>,
    pub play_count: i64,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub cache_path: String,
    pub cached_at: Option<i64>,
    pub last_played_at: Option<i64>,
}

pub(crate) const TRACK_INFO_COLUMNS: &str = r#"
t.bvid, t.cid, t.title, t.cover_url, t.author, t.duration,
t.audio_id, t.codec, t.cache_path, t.cached_at,
t.last_played_at, t.play_count
"#;

pub(crate) fn row_to_track_info(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<TrackInfo> {
    Ok(TrackInfo {
        bvid: row.get(0)?,
        cid: row.get(1)?,
        title: row.get(2)?,
        cover_url: row.get(3)?,
        author: row.get(4)?,
        duration: row.get(5)?,
        audio_id: row.get(6)?,
        codec: row.get(7)?,
        cache_path: row.get(8)?,
        cached_at: row.get(9)?,
        last_played_at: row.get(10)?,
        play_count: row.get(11)?,
    })
}

pub fn upsert_track(connection: &Connection, track: &TrackRecord) -> Result<(), ApiError> {
    connection.execute(
        r#"
        INSERT INTO tracks
          (bvid, cid, title, cover_url, author, duration, audio_id, codec, cache_path, cached_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(bvid, cid) DO UPDATE SET
          title = excluded.title,
          cover_url = excluded.cover_url,
          author = excluded.author,
          duration = excluded.duration,
          audio_id = excluded.audio_id,
          codec = excluded.codec,
          cache_path = COALESCE(excluded.cache_path, tracks.cache_path)
        "#,
        rusqlite::params![
            track.bvid,
            track.cid,
            track.title,
            track.cover_url,
            track.author,
            track.duration,
            track.audio_id,
            track.codec,
            track.cache_path,
            track.cached_at,
        ],
    )?;
    Ok(())
}

pub fn mark_cached(
    connection: &Connection,
    bvid: &str,
    cid: u64,
    cache_path: &str,
    cached_at: i64,
) -> Result<(), ApiError> {
    connection.execute(
        "UPDATE tracks SET cache_path = ?3, cached_at = ?4 WHERE bvid = ?1 AND cid = ?2",
        rusqlite::params![bvid, cid, cache_path, cached_at],
    )?;
    Ok(())
}

pub fn get_track_by_bvid(
    connection: &Connection,
    bvid: &str,
    cid: u64,
) -> Result<Option<TrackRecord>, ApiError> {
    let mut statement = connection.prepare(
        r#"
        SELECT bvid, cid, title, cover_url, author, duration, audio_id, codec, cache_path, cached_at
        FROM tracks WHERE bvid = ?1 AND cid = ?2
        "#,
    )?;
    let mut rows = statement.query(rusqlite::params![bvid, cid])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(TrackRecord {
        bvid: row.get(0)?,
        cid: row.get(1)?,
        title: row.get(2)?,
        cover_url: row.get(3)?,
        author: row.get(4)?,
        duration: row.get(5)?,
        audio_id: row.get(6)?,
        codec: row.get(7)?,
        cache_path: row.get(8)?,
        cached_at: row.get(9)?,
    }))
}

pub fn get_track_id(
    connection: &Connection,
    bvid: &str,
    cid: u64,
) -> Result<Option<i64>, ApiError> {
    Ok(connection
        .query_row(
            "SELECT id FROM tracks WHERE bvid = ?1 AND cid = ?2",
            rusqlite::params![bvid, cid],
            |row| row.get(0),
        )
        .optional()?)
}

#[allow(dead_code)]
pub fn get_track_info_by_bvid(
    connection: &Connection,
    bvid: &str,
    cid: u64,
) -> Result<Option<TrackInfo>, ApiError> {
    let sql = format!(
        "SELECT {TRACK_INFO_COLUMNS} FROM tracks t WHERE t.bvid = ?1 AND t.cid = ?2"
    );
    Ok(connection
        .query_row(&sql, rusqlite::params![bvid, cid], row_to_track_info)
        .optional()?)
}

/// 播放成功时更新最近播放时间与播放次数。
pub fn touch_played(
    connection: &Connection,
    bvid: &str,
    cid: u64,
    played_at: i64,
) -> Result<(), ApiError> {
    connection.execute(
        r#"
        UPDATE tracks
        SET last_played_at = ?3, play_count = play_count + 1
        WHERE bvid = ?1 AND cid = ?2
        "#,
        rusqlite::params![bvid, cid, played_at],
    )?;
    Ok(())
}

pub fn list_cached_tracks(
    connection: &Connection,
) -> Result<Vec<TrackInfo>, ApiError> {
    let sql = format!(
        "SELECT {TRACK_INFO_COLUMNS} FROM tracks t
         WHERE t.cached_at IS NOT NULL
         ORDER BY t.cached_at DESC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], row_to_track_info)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn list_cache_entries(
    connection: &Connection,
) -> Result<Vec<CacheEntry>, ApiError> {
    let mut statement = connection.prepare(
        r#"
        SELECT cache_path, cached_at, last_played_at
        FROM tracks WHERE cache_path IS NOT NULL
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(CacheEntry {
            cache_path: row.get(0)?,
            cached_at: row.get(1)?,
            last_played_at: row.get(2)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn clear_cache_entries(
    connection: &Connection,
    paths: &[String],
) -> Result<(), ApiError> {
    for path in paths {
        connection.execute(
            r#"
            UPDATE tracks
            SET cache_path = NULL, cached_at = NULL
            WHERE cache_path = ?1
            "#,
            rusqlite::params![path],
        )?;
    }
    Ok(())
}

pub fn update_cache_paths(
    connection: &Connection,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<(), ApiError> {
    connection.execute(
        r#"
        UPDATE tracks
        SET cache_path = ?2 || substr(cache_path, length(?1) + 1)
        WHERE cache_path LIKE ?1 || '%'
        "#,
        rusqlite::params![old_prefix, new_prefix],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_get_mark_roundtrip() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        connection
            .execute_batch(
                r#"
                CREATE TABLE tracks (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  bvid TEXT NOT NULL,
                  cid INTEGER,
                  title TEXT NOT NULL,
                  cover_url TEXT,
                  author TEXT,
                  duration INTEGER,
                  audio_id INTEGER,
                  codec TEXT,
                  cache_path TEXT,
                  cached_at INTEGER,
                  last_played_at INTEGER,
                  play_count INTEGER DEFAULT 0,
                  UNIQUE(bvid, cid)
                )
                "#,
            )
            .expect("建表应成功");
        let track = TrackRecord {
            bvid: "BV1TEST".to_string(),
            cid: 123,
            title: "测试曲目".to_string(),
            cover_url: "https://example.com/c.jpg".to_string(),
            author: "测试UP".to_string(),
            duration: 240,
            audio_id: 30280,
            codec: "mp4a".to_string(),
            cache_path: Some("C:\\cache\\BV1TEST_30280.m4a".to_string()),
            cached_at: None,
        };
        upsert_track(&connection, &track).expect("upsert 应成功");
        let loaded = get_track_by_bvid(&connection, "BV1TEST", 123)
            .expect("查询应成功")
            .expect("记录应存在");
        assert_eq!(loaded.title, "测试曲目");
        assert_eq!(loaded.audio_id, 30280);
        assert!(loaded.cached_at.is_none());

        mark_cached(&connection, "BV1TEST", 123, "C:\\cache\\BV1TEST_30280.m4a", 123456)
            .expect("mark_cached 应成功");
        let loaded = get_track_by_bvid(&connection, "BV1TEST", 123)
            .expect("查询应成功")
            .expect("记录应存在");
        assert_eq!(loaded.cached_at, Some(123456));
    }

    #[test]
    fn touch_and_cached_list() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        connection
            .execute_batch(
                r#"
                CREATE TABLE tracks (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  bvid TEXT NOT NULL,
                  cid INTEGER, title TEXT NOT NULL, cover_url TEXT,
                  author TEXT, duration INTEGER, audio_id INTEGER,
                  codec TEXT, cache_path TEXT, cached_at INTEGER,
                  last_played_at INTEGER, play_count INTEGER DEFAULT 0,
                  UNIQUE(bvid, cid)
                )
                "#,
            )
            .expect("建表应成功");
        let track = TrackRecord {
            bvid: "BV1CACHE".to_string(),
            cid: 1,
            title: "已缓存曲目".to_string(),
            cover_url: String::new(),
            author: "UP".to_string(),
            duration: 120,
            audio_id: 30280,
            codec: "mp4a".to_string(),
            cache_path: Some("C:\\cache\\x.m4a".to_string()),
            cached_at: Some(1000),
        };
        upsert_track(&connection, &track).expect("upsert 应成功");
        touch_played(&connection, "BV1CACHE", 1, 2000).expect("touch 应成功");
        let info = get_track_info_by_bvid(&connection, "BV1CACHE", 1)
            .expect("查询应成功")
            .expect("记录应存在");
        assert_eq!(info.last_played_at, Some(2000));
        assert_eq!(info.play_count, 1);
        let cached = list_cached_tracks(&connection).expect("列表应成功");
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].cached_at, Some(1000));
    }
}
