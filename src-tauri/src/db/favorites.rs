use rusqlite::Connection;
use serde::Serialize;

use super::tracks::{row_to_track_info, TrackInfo};
use crate::models::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub track: TrackInfo,
    pub created_at: i64,
}

pub fn add_favorite(
    connection: &Connection,
    bvid: &str,
    cid: u64,
    created_at: i64,
) -> Result<(), ApiError> {
    let track_id = super::tracks::get_track_id(connection, bvid, cid)?
        .ok_or_else(|| ApiError::Invalid("曲目尚未记录，无法收藏".to_string()))?;
    connection.execute(
        r#"
        INSERT OR IGNORE INTO favorites (track_id, created_at)
        VALUES (?1, ?2)
        "#,
        rusqlite::params![track_id, created_at],
    )?;
    Ok(())
}

pub fn remove_favorite(
    connection: &Connection,
    bvid: &str,
    cid: u64,
) -> Result<(), ApiError> {
    connection.execute(
        r#"
        DELETE FROM favorites
        WHERE track_id = (SELECT id FROM tracks WHERE bvid = ?1 AND cid = ?2)
        "#,
        rusqlite::params![bvid, cid],
    )?;
    Ok(())
}

pub fn list_favorites(
    connection: &Connection,
) -> Result<Vec<FavoriteItem>, ApiError> {
    let sql = format!(
        r#"
        SELECT {}, f.created_at
        FROM favorites f
        JOIN tracks t ON t.id = f.track_id
        ORDER BY f.created_at DESC
        "#,
        super::tracks::TRACK_INFO_COLUMNS
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        let track = row_to_track_info(row)?;
        Ok((track, row.get(12)?))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (track, created_at) = row?;
        items.push(FavoriteItem { track, created_at });
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tracks::{upsert_track, TrackRecord};

    fn seed(connection: &Connection, bvid: &str) {
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS tracks (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  bvid TEXT NOT NULL, cid INTEGER, title TEXT NOT NULL,
                  cover_url TEXT, author TEXT, duration INTEGER,
                  audio_id INTEGER, codec TEXT, cache_path TEXT,
                  cached_at INTEGER, last_played_at INTEGER,
                  play_count INTEGER DEFAULT 0,
                  UNIQUE(bvid, cid)
                );
                CREATE TABLE IF NOT EXISTS favorites (
                  track_id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL
                );
                "#,
            )
            .expect("建表应成功");
        upsert_track(
            connection,
            &TrackRecord {
                bvid: bvid.to_string(),
                cid: 1,
                title: "收藏曲目".to_string(),
                cover_url: String::new(),
                author: "UP".to_string(),
                duration: 100,
                audio_id: 30280,
                codec: "mp4a".to_string(),
                cache_path: None,
                cached_at: None,
            },
        )
        .expect("upsert 应成功");
    }

    #[test]
    fn favorite_crud_and_list() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        seed(&connection, "BV1FAV");
        add_favorite(&connection, "BV1FAV", 1, 111).expect("收藏应成功");
        add_favorite(&connection, "BV1FAV", 1, 222).expect("重复收藏应忽略");
        let items = list_favorites(&connection).expect("列表应成功");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].created_at, 111);
        assert_eq!(items[0].track.bvid, "BV1FAV");
        remove_favorite(&connection, "BV1FAV", 1).expect("取消收藏应成功");
        assert!(list_favorites(&connection).expect("列表应成功").is_empty());
    }
}
