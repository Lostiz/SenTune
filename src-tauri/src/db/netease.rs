//! 网易云曲目与资料库（收藏 / 历史）数据层。
//!
//! 与本地音乐同思路：独立表（`netease_tracks` / `netease_favorites` /
//! `netease_history`），不改动 B 站 tracks/favorites/history 的既有结构，
//! 迁移风险最低。

use rusqlite::Connection;
use serde::Serialize;

use crate::models::ApiError;

#[derive(Debug, Clone)]
pub struct NeteaseTrackRecord {
    pub song_id: u64,
    pub title: String,
    pub artist: String,
    pub album_name: String,
    pub cover_url: String,
    pub duration_ms: u64,
    pub fee: u64,
    pub cache_path: Option<String>,
    pub cached_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseTrackInfo {
    pub song_id: u64,
    pub title: String,
    pub artist: String,
    pub album_name: String,
    pub cover_url: String,
    pub duration_ms: u64,
    pub fee: u64,
    pub cache_path: Option<String>,
    pub cached_at: Option<i64>,
    pub last_played_at: Option<i64>,
    pub play_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseFavoriteItem {
    pub track: NeteaseTrackInfo,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseHistoryItem {
    pub track: NeteaseTrackInfo,
    pub played_at: i64,
}

const TRACK_INFO_COLUMNS: &str = r#"
t.song_id, t.title, t.artist, t.album_name, t.cover_url, t.duration_ms, t.fee,
t.cache_path, t.cached_at, t.last_played_at, t.play_count
"#;

fn row_to_track_info(row: &rusqlite::Row<'_>) -> rusqlite::Result<NeteaseTrackInfo> {
    Ok(NeteaseTrackInfo {
        song_id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        album_name: row.get(3)?,
        cover_url: row.get(4)?,
        duration_ms: row.get(5)?,
        fee: row.get(6)?,
        cache_path: row.get(7)?,
        cached_at: row.get(8)?,
        last_played_at: row.get(9)?,
        play_count: row.get(10)?,
    })
}

/// 写入/更新网易云曲目元数据（song_id 唯一）。
pub fn upsert_track(
    connection: &Connection,
    track: &NeteaseTrackRecord,
) -> Result<(), ApiError> {
    connection.execute(
        r#"
        INSERT INTO netease_tracks
          (song_id, title, artist, album_name, cover_url, duration_ms, fee, cache_path, cached_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(song_id) DO UPDATE SET
          title = excluded.title,
          artist = excluded.artist,
          album_name = excluded.album_name,
          cover_url = excluded.cover_url,
          duration_ms = excluded.duration_ms,
          fee = excluded.fee,
          cache_path = COALESCE(excluded.cache_path, netease_tracks.cache_path)
        "#,
        rusqlite::params![
            track.song_id,
            track.title,
            track.artist,
            track.album_name,
            track.cover_url,
            track.duration_ms,
            track.fee,
            track.cache_path,
            track.cached_at,
        ],
    )?;
    Ok(())
}

fn track_id(connection: &Connection, song_id: u64) -> Result<Option<i64>, ApiError> {
    use rusqlite::OptionalExtension;
    Ok(connection
        .query_row(
            "SELECT id FROM netease_tracks WHERE song_id = ?1",
            rusqlite::params![song_id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn add_favorite(
    connection: &Connection,
    track: &NeteaseTrackRecord,
    created_at: i64,
) -> Result<(), ApiError> {
    upsert_track(connection, track)?;
    let id = track_id(connection, track.song_id)?
        .ok_or_else(|| ApiError::Invalid("曲目写入失败".to_string()))?;
    connection.execute(
        "INSERT OR IGNORE INTO netease_favorites (track_id, created_at) VALUES (?1, ?2)",
        rusqlite::params![id, created_at],
    )?;
    Ok(())
}

pub fn remove_favorite(connection: &Connection, song_id: u64) -> Result<(), ApiError> {
    connection.execute(
        r#"
        DELETE FROM netease_favorites
        WHERE track_id = (SELECT id FROM netease_tracks WHERE song_id = ?1)
        "#,
        rusqlite::params![song_id],
    )?;
    Ok(())
}

pub fn list_favorites(
    connection: &Connection,
) -> Result<Vec<NeteaseFavoriteItem>, ApiError> {
    let sql = format!(
        r#"
        SELECT {}, f.created_at
        FROM netease_favorites f
        JOIN netease_tracks t ON t.id = f.track_id
        ORDER BY f.created_at DESC
        "#,
        TRACK_INFO_COLUMNS
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        let track = row_to_track_info(row)?;
        Ok((track, row.get(11)?))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (track, created_at) = row?;
        items.push(NeteaseFavoriteItem { track, created_at });
    }
    Ok(items)
}

/// 写入历史并更新播放统计；同一曲目重播只更新时间戳。
pub fn add_history(
    connection: &Connection,
    track: &NeteaseTrackRecord,
    played_at: i64,
) -> Result<(), ApiError> {
    upsert_track(connection, track)?;
    let id = track_id(connection, track.song_id)?
        .ok_or_else(|| ApiError::Invalid("曲目写入失败".to_string()))?;
    connection.execute(
        r#"
        INSERT INTO netease_history (track_id, played_at) VALUES (?1, ?2)
        ON CONFLICT(track_id) DO UPDATE SET played_at = excluded.played_at
        "#,
        rusqlite::params![id, played_at],
    )?;
    connection.execute(
        r#"
        UPDATE netease_tracks
        SET last_played_at = ?2, play_count = play_count + 1
        WHERE id = ?1
        "#,
        rusqlite::params![id, played_at],
    )?;
    Ok(())
}

pub fn list_history(connection: &Connection) -> Result<Vec<NeteaseHistoryItem>, ApiError> {
    let sql = format!(
        r#"
        SELECT {}, h.played_at
        FROM netease_history h
        JOIN netease_tracks t ON t.id = h.track_id
        ORDER BY h.played_at DESC, h.id DESC
        LIMIT 300
        "#,
        TRACK_INFO_COLUMNS
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        let track = row_to_track_info(row)?;
        Ok((track, row.get(11)?))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (track, played_at) = row?;
        items.push(NeteaseHistoryItem { track, played_at });
    }
    Ok(items)
}

pub fn clear_history(connection: &Connection) -> Result<(), ApiError> {
    connection.execute("DELETE FROM netease_history", [])?;
    Ok(())
}

pub fn mark_cached(
    connection: &Connection,
    song_id: u64,
    cache_path: &str,
    cached_at: i64,
) -> Result<(), ApiError> {
    connection.execute(
        "UPDATE netease_tracks SET cache_path = ?2, cached_at = ?3 WHERE song_id = ?1",
        rusqlite::params![song_id, cache_path, cached_at],
    )?;
    Ok(())
}

pub fn get_track_by_song(
    connection: &Connection,
    song_id: u64,
) -> Result<Option<NeteaseTrackRecord>, ApiError> {
    use rusqlite::OptionalExtension;
    Ok(connection
        .query_row(
            r#"
            SELECT song_id, title, artist, album_name, cover_url, duration_ms, fee, cache_path, cached_at
            FROM netease_tracks WHERE song_id = ?1
            "#,
            rusqlite::params![song_id],
            |row| {
                Ok(NeteaseTrackRecord {
                    song_id: row.get(0)?,
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    album_name: row.get(3)?,
                    cover_url: row.get(4)?,
                    duration_ms: row.get(5)?,
                    fee: row.get(6)?,
                    cache_path: row.get(7)?,
                    cached_at: row.get(8)?,
                })
            },
        )
        .optional()?)
}

#[derive(Debug, Clone)]
pub struct NeteaseCacheEntry {
    pub cache_path: String,
    pub cached_at: Option<i64>,
    pub last_played_at: Option<i64>,
}

pub fn list_cache_entries(
    connection: &Connection,
) -> Result<Vec<NeteaseCacheEntry>, ApiError> {
    let mut statement = connection.prepare(
        r#"
        SELECT cache_path, cached_at, last_played_at
        FROM netease_tracks WHERE cache_path IS NOT NULL
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(NeteaseCacheEntry {
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
            UPDATE netease_tracks
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
        UPDATE netease_tracks
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

    fn seed_tables(connection: &Connection) {
        connection
            .execute_batch(
                r#"
                CREATE TABLE netease_tracks (
                  id             INTEGER PRIMARY KEY AUTOINCREMENT,
                  song_id        INTEGER NOT NULL UNIQUE,
                  title          TEXT NOT NULL,
                  artist         TEXT NOT NULL DEFAULT '',
                  album_name     TEXT NOT NULL DEFAULT '',
                  cover_url      TEXT NOT NULL DEFAULT '',
                  duration_ms    INTEGER NOT NULL DEFAULT 0,
                  fee            INTEGER NOT NULL DEFAULT 0,
                  cache_path     TEXT,
                  cached_at      INTEGER,
                  last_played_at INTEGER,
                  play_count     INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE netease_favorites (
                  track_id   INTEGER PRIMARY KEY,
                  created_at INTEGER NOT NULL
                );
                CREATE TABLE netease_history (
                  id        INTEGER PRIMARY KEY AUTOINCREMENT,
                  track_id  INTEGER NOT NULL,
                  played_at INTEGER NOT NULL
                );
                CREATE UNIQUE INDEX idx_netease_history_track_id
                  ON netease_history(track_id);
                "#,
            )
            .expect("建表应成功");
    }

    fn sample(song_id: u64) -> NeteaseTrackRecord {
        NeteaseTrackRecord {
            song_id,
            title: format!("网易云曲目 {song_id}"),
            artist: "歌手".to_string(),
            album_name: "专辑".to_string(),
            cover_url: "https://p2.music.126.net/x.jpg".to_string(),
            duration_ms: 200_000,
            fee: 0,
            cache_path: None,
            cached_at: None,
        }
    }

    #[test]
    fn favorite_and_history_flow() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        seed_tables(&connection);

        // 收藏：重复收藏只保留一条。
        add_favorite(&connection, &sample(1001), 111).expect("收藏应成功");
        add_favorite(&connection, &sample(1001), 222).expect("重复收藏应忽略");
        let favorites = list_favorites(&connection).expect("列表应成功");
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].created_at, 111);
        assert_eq!(favorites[0].track.song_id, 1001);
        assert_eq!(favorites[0].track.title, "网易云曲目 1001");

        // 历史：重播更新时间戳且只保留一条。
        add_history(&connection, &sample(1001), 10).expect("写历史应成功");
        add_history(&connection, &sample(1001), 20).expect("写历史应成功");
        add_history(&connection, &sample(2002), 30).expect("写历史应成功");
        let history = list_history(&connection).expect("列表应成功");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].track.song_id, 2002, "最新播放在前");
        assert_eq!(history[1].played_at, 20, "重播更新时间戳");
        assert_eq!(history[1].track.play_count, 2);

        // 取消收藏与清空历史。
        remove_favorite(&connection, 1001).expect("取消收藏应成功");
        assert!(list_favorites(&connection).expect("列表应成功").is_empty());
        clear_history(&connection).expect("清空应成功");
        assert!(list_history(&connection).expect("列表应成功").is_empty());
    }
}
