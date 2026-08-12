use std::path::Path;
use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

use crate::models::ApiError;

pub mod favorites;
pub mod history;
pub mod local;
pub mod playlists;
pub mod settings;
pub mod tracks;

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tracks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  bvid          TEXT NOT NULL,
  cid           INTEGER,
  title         TEXT NOT NULL,
  cover_url     TEXT,
  author        TEXT,
  duration      INTEGER,
  audio_id      INTEGER,
  codec         TEXT,
  cache_path    TEXT,
  cached_at     INTEGER,
  last_played_at INTEGER,
  play_count    INTEGER DEFAULT 0,
  UNIQUE(bvid, cid)
);
CREATE INDEX IF NOT EXISTS idx_tracks_bvid ON tracks(bvid);

CREATE TABLE IF NOT EXISTS playlists (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL,
  created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
  playlist_id INTEGER NOT NULL,
  track_id    INTEGER NOT NULL,
  position    INTEGER NOT NULL,
  PRIMARY KEY (playlist_id, track_id)
);
CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist
  ON playlist_tracks(playlist_id, position);

CREATE TABLE IF NOT EXISTS favorites (
  track_id    INTEGER PRIMARY KEY,
  created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS history (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  track_id    INTEGER NOT NULL,
  played_at   INTEGER NOT NULL
);
DELETE FROM history
WHERE id NOT IN (SELECT MAX(id) FROM history GROUP BY track_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_history_track_id ON history(track_id);
CREATE INDEX IF NOT EXISTS idx_history_played_at ON history(played_at);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_folders (
  id       INTEGER PRIMARY KEY AUTOINCREMENT,
  path     TEXT NOT NULL UNIQUE,
  added_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS local_tracks (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  path           TEXT NOT NULL UNIQUE,
  folder_id      INTEGER,
  title          TEXT NOT NULL,
  artist         TEXT NOT NULL DEFAULT '',
  album          TEXT NOT NULL DEFAULT '',
  duration       INTEGER NOT NULL DEFAULT 0,
  codec          TEXT NOT NULL DEFAULT '',
  size           INTEGER NOT NULL DEFAULT 0,
  modified_at    INTEGER NOT NULL DEFAULT 0,
  cover_path     TEXT,
  added_at       INTEGER NOT NULL,
  last_played_at INTEGER,
  play_count     INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_local_tracks_folder ON local_tracks(folder_id);
CREATE INDEX IF NOT EXISTS idx_local_tracks_title ON local_tracks(title);

CREATE TABLE IF NOT EXISTS local_favorites (
  track_id   INTEGER PRIMARY KEY,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS local_history (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  track_id  INTEGER NOT NULL,
  played_at INTEGER NOT NULL
);
DELETE FROM local_history
WHERE id NOT IN (SELECT MAX(id) FROM local_history GROUP BY track_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_local_history_track_id
  ON local_history(track_id);
"#;

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

pub fn init(path: &Path) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let connection = Connection::open(path)?;
    migrate_tracks_schema(&connection)?;
    connection.execute_batch(SCHEMA)?;
    let _ = DB.set(Mutex::new(connection));
    Ok(())
}

/// 旧版 tracks 以 bvid 为唯一键，多 P 合辑需要改为 (bvid, cid)。
fn migrate_tracks_schema(connection: &Connection) -> Result<(), ApiError> {
    use rusqlite::OptionalExtension;
    let sql: Option<String> = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tracks'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    // SQLite 保存的建表语句可能含多个连续空格，先归一化再判断：
    // 只要没有 (bvid, cid) 唯一约束就重建。
    let needs_rebuild = sql.as_deref().is_some_and(|sql| {
        let normalized = sql.split_whitespace().collect::<Vec<_>>().join(" ");
        !normalized.contains("UNIQUE(bvid, cid)")
    });
    if needs_rebuild {
        connection.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE tracks_new (
              id            INTEGER PRIMARY KEY AUTOINCREMENT,
              bvid          TEXT NOT NULL,
              cid           INTEGER,
              title         TEXT NOT NULL,
              cover_url     TEXT,
              author        TEXT,
              duration      INTEGER,
              audio_id      INTEGER,
              codec         TEXT,
              cache_path    TEXT,
              cached_at     INTEGER,
              last_played_at INTEGER,
              play_count    INTEGER DEFAULT 0,
              UNIQUE(bvid, cid)
            );
            INSERT INTO tracks_new
              (id, bvid, cid, title, cover_url, author, duration,
               audio_id, codec, cache_path, cached_at, last_played_at, play_count)
            SELECT
              id, bvid, cid, title, cover_url, author, duration,
              audio_id, codec, cache_path, cached_at, last_played_at, play_count
            FROM tracks;
            DROP TABLE tracks;
            ALTER TABLE tracks_new RENAME TO tracks;
            COMMIT;
            "#,
        )?;
    }
    Ok(())
}

pub fn connection() -> &'static Mutex<Connection> {
    DB.get().expect("数据库尚未初始化")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tracks::{upsert_track, TrackRecord};

    #[test]
    fn migrate_rebuilds_old_tracks_schema() {
        let connection =
            rusqlite::Connection::open_in_memory().expect("内存数据库应可打开");
        // 旧版建表语句（带多空格，唯一约束为 bvid）。
        connection
            .execute_batch(
                r#"
                CREATE TABLE tracks (
                  id            INTEGER PRIMARY KEY AUTOINCREMENT,
                  bvid          TEXT NOT NULL UNIQUE,
                  cid           INTEGER,
                  title         TEXT NOT NULL,
                  cover_url     TEXT,
                  author        TEXT,
                  duration      INTEGER,
                  audio_id      INTEGER,
                  codec         TEXT,
                  cache_path    TEXT,
                  cached_at     INTEGER,
                  last_played_at INTEGER,
                  play_count    INTEGER DEFAULT 0
                );
                INSERT INTO tracks (bvid, cid, title)
                VALUES ('BV1OLD', 1, '旧曲目');
                "#,
            )
            .expect("旧表应可创建");

        migrate_tracks_schema(&connection).expect("迁移应成功");

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .expect("统计应成功");
        assert_eq!(count, 1, "旧数据应保留");

        // 迁移后 (bvid, cid) 唯一约束应可用。
        let track = TrackRecord {
            bvid: "BV1OLD".to_string(),
            cid: 1,
            title: "旧曲目".to_string(),
            cover_url: String::new(),
            author: String::new(),
            duration: 0,
            audio_id: 0,
            codec: "mp4a".to_string(),
            cache_path: None,
            cached_at: None,
        };
        upsert_track(&connection, &track).expect("ON CONFLICT(bvid, cid) 应成功");

        let track2 = TrackRecord {
            bvid: "BV1OLD".to_string(),
            cid: 2,
            title: "第二集".to_string(),
            cover_url: String::new(),
            author: String::new(),
            duration: 0,
            audio_id: 0,
            codec: "mp4a".to_string(),
            cache_path: None,
            cached_at: None,
        };
        upsert_track(&connection, &track2).expect("同 bvid 不同 cid 应可插入");
    }
}
