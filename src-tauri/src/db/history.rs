use rusqlite::Connection;
use serde::Serialize;

use super::tracks::{row_to_track_info, TrackInfo};
use crate::models::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub track: TrackInfo,
    pub played_at: i64,
}

/// 写入历史并更新曲目播放统计（last_played_at / play_count）。
pub fn add_history(
    connection: &Connection,
    bvid: &str,
    cid: u64,
    played_at: i64,
) -> Result<(), ApiError> {
    let track_id = super::tracks::get_track_id(connection, bvid, cid)?
        .ok_or_else(|| ApiError::Invalid("曲目尚未记录，无法写入历史".to_string()))?;
    connection.execute(
        r#"
        INSERT INTO history (track_id, played_at) VALUES (?1, ?2)
        ON CONFLICT(track_id) DO UPDATE SET played_at = excluded.played_at
        "#,
        rusqlite::params![track_id, played_at],
    )?;
    super::tracks::touch_played(connection, bvid, cid, played_at)?;
    Ok(())
}

pub fn list_history(connection: &Connection) -> Result<Vec<HistoryItem>, ApiError> {
    let sql = format!(
        r#"
        SELECT {}, h.played_at
        FROM history h
        JOIN tracks t ON t.id = h.track_id
        ORDER BY h.played_at DESC, h.id DESC
        LIMIT 300
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
        let (track, played_at) = row?;
        items.push(HistoryItem { track, played_at });
    }
    Ok(items)
}

pub fn clear_history(connection: &Connection) -> Result<(), ApiError> {
    connection.execute("DELETE FROM history", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tracks::{upsert_track, TrackRecord};

    #[test]
    fn history_flow() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        connection
            .execute_batch(
                r#"
                CREATE TABLE tracks (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  bvid TEXT NOT NULL, cid INTEGER, title TEXT NOT NULL,
                  cover_url TEXT, author TEXT, duration INTEGER,
                  audio_id INTEGER, codec TEXT, cache_path TEXT,
                  cached_at INTEGER, last_played_at INTEGER,
                  play_count INTEGER DEFAULT 0,
                  UNIQUE(bvid, cid)
                );
                CREATE TABLE history (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  track_id INTEGER NOT NULL, played_at INTEGER NOT NULL
                );
                CREATE UNIQUE INDEX idx_history_track_id ON history(track_id);
                "#,
            )
            .expect("建表应成功");
        upsert_track(
            &connection,
            &TrackRecord {
                bvid: "BV1HIST".to_string(),
                cid: 1,
                title: "历史曲目".to_string(),
                cover_url: String::new(),
                author: "UP".to_string(),
                duration: 90,
                audio_id: 30280,
                codec: "mp4a".to_string(),
                cache_path: None,
                cached_at: None,
            },
        )
        .expect("upsert 应成功");
        add_history(&connection, "BV1HIST", 1, 10).expect("写历史应成功");
        add_history(&connection, "BV1HIST", 1, 20).expect("写历史应成功");
        let items = list_history(&connection).expect("列表应成功");
        assert_eq!(items.len(), 1, "同一曲目重播应只保留一条历史");
        assert_eq!(items[0].played_at, 20, "重播应更新时间戳");
        let info = crate::db::tracks::get_track_info_by_bvid(&connection, "BV1HIST", 1)
            .expect("查询应成功")
            .expect("记录应存在");
        assert_eq!(info.play_count, 2);
        clear_history(&connection).expect("清空应成功");
        assert!(list_history(&connection).expect("列表应成功").is_empty());
    }
}
