use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use super::tracks::{row_to_track_info, TrackInfo};
use crate::models::ApiError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub tracks: Vec<TrackInfo>,
}

pub fn create_playlist(
    connection: &Connection,
    name: &str,
    created_at: i64,
) -> Result<i64, ApiError> {
    connection.execute(
        "INSERT INTO playlists (name, created_at) VALUES (?1, ?2)",
        rusqlite::params![name, created_at],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn rename_playlist(
    connection: &Connection,
    id: i64,
    name: &str,
) -> Result<(), ApiError> {
    connection.execute(
        "UPDATE playlists SET name = ?2 WHERE id = ?1",
        rusqlite::params![id, name],
    )?;
    Ok(())
}

pub fn delete_playlist(
    connection: &Connection,
    id: i64,
) -> Result<(), ApiError> {
    connection.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
        rusqlite::params![id],
    )?;
    connection.execute(
        "DELETE FROM playlists WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

pub fn list_playlists(
    connection: &Connection,
) -> Result<Vec<PlaylistSummary>, ApiError> {
    let mut statement = connection.prepare(
        r#"
        SELECT p.id, p.name, p.created_at,
               (SELECT COUNT(*) FROM playlist_tracks pt WHERE pt.playlist_id = p.id)
        FROM playlists p
        ORDER BY p.created_at DESC
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(PlaylistSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            track_count: row.get(3)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn get_playlist_name(
    connection: &Connection,
    id: i64,
) -> Result<Option<String>, ApiError> {
    Ok(connection
        .query_row(
            "SELECT name FROM playlists WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn add_track(
    connection: &Connection,
    playlist_id: i64,
    bvid: &str,
    cid: u64,
) -> Result<(), ApiError> {
    let Some(track_id) = super::tracks::get_track_id(connection, bvid, cid)? else {
        return Ok(());
    };
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM playlist_tracks
             WHERE playlist_id = ?1 AND track_id = ?2)",
            rusqlite::params![playlist_id, track_id],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if exists {
        return Ok(());
    }
    let position: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1
             FROM playlist_tracks WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    connection.execute(
        "INSERT INTO playlist_tracks (playlist_id, track_id, position)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![playlist_id, track_id, position],
    )?;
    Ok(())
}

pub fn remove_track(
    connection: &Connection,
    playlist_id: i64,
    bvid: &str,
    cid: u64,
) -> Result<(), ApiError> {
    connection.execute(
        r#"
        DELETE FROM playlist_tracks
        WHERE playlist_id = ?1 AND track_id =
          (SELECT id FROM tracks WHERE bvid = ?2 AND cid = ?3)
        "#,
        rusqlite::params![playlist_id, bvid, cid],
    )?;
    renumber(connection, playlist_id)?;
    Ok(())
}

fn ordered_track_ids(
    connection: &Connection,
    playlist_id: i64,
) -> Result<Vec<i64>, ApiError> {
    let mut statement = connection.prepare(
        "SELECT track_id FROM playlist_tracks
         WHERE playlist_id = ?1 ORDER BY position, rowid",
    )?;
    let rows = statement.query_map(rusqlite::params![playlist_id], |row| row.get(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

fn renumber(connection: &Connection, playlist_id: i64) -> Result<(), ApiError> {
    let ids = ordered_track_ids(connection, playlist_id)?;
    let transaction = connection.unchecked_transaction()?;
    for (index, track_id) in ids.iter().enumerate() {
        transaction.execute(
            "UPDATE playlist_tracks SET position = ?3
             WHERE playlist_id = ?1 AND track_id = ?2",
            rusqlite::params![playlist_id, track_id, index as i64],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn move_track(
    connection: &Connection,
    playlist_id: i64,
    bvid: &str,
    cid: u64,
    to_position: i64,
) -> Result<(), ApiError> {
    let Some(track_id) = super::tracks::get_track_id(connection, bvid, cid)? else {
        return Ok(());
    };
    let mut ids = ordered_track_ids(connection, playlist_id)?;
    let Some(from) = ids.iter().position(|id| *id == track_id) else {
        return Ok(());
    };
    ids.remove(from);
    let to = (to_position as usize).min(ids.len());
    ids.insert(to, track_id);
    let transaction = connection.unchecked_transaction()?;
    for (index, id) in ids.iter().enumerate() {
        transaction.execute(
            "UPDATE playlist_tracks SET position = ?3
             WHERE playlist_id = ?1 AND track_id = ?2",
            rusqlite::params![playlist_id, id, index as i64],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn list_playlist_tracks(
    connection: &Connection,
    playlist_id: i64,
) -> Result<Vec<TrackInfo>, ApiError> {
    let sql = format!(
        r#"
        SELECT {}, NULL
        FROM playlist_tracks pt
        JOIN tracks t ON t.id = pt.track_id
        WHERE pt.playlist_id = ?1
        ORDER BY pt.position, pt.rowid
        "#,
        super::tracks::TRACK_INFO_COLUMNS
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params![playlist_id], row_to_track_info)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tracks::{upsert_track, TrackRecord};

    fn seed(connection: &Connection) {
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
                CREATE TABLE playlists (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  name TEXT NOT NULL, created_at INTEGER NOT NULL
                );
                CREATE TABLE playlist_tracks (
                  playlist_id INTEGER NOT NULL,
                  track_id INTEGER NOT NULL,
                  position INTEGER NOT NULL,
                  PRIMARY KEY (playlist_id, track_id)
                );
                "#,
            )
            .expect("建表应成功");
        for bvid in ["BV1PLA", "BV1PLB", "BV1PLC"] {
            upsert_track(
                connection,
                &TrackRecord {
                    bvid: bvid.to_string(),
                    cid: 1,
                    title: format!("曲目 {bvid}"),
                    cover_url: String::new(),
                    author: "UP".to_string(),
                    duration: 60,
                    audio_id: 30280,
                    codec: "mp4a".to_string(),
                    cache_path: None,
                    cached_at: None,
                },
            )
            .expect("upsert 应成功");
        }
    }

    #[test]
    fn playlist_crud_and_reorder() {
        let connection =
            Connection::open_in_memory().expect("内存数据库应可打开");
        seed(&connection);
        let id = create_playlist(&connection, "我的歌单", 1).expect("创建应成功");
        for bvid in ["BV1PLA", "BV1PLB", "BV1PLC"] {
            add_track(&connection, id, bvid, 1).expect("加曲应成功");
        }
        add_track(&connection, id, "BV1PLA", 1).expect("重复加曲应忽略");
        let summaries = list_playlists(&connection).expect("列表应成功");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].track_count, 3);

        move_track(&connection, id, "BV1PLC", 1, 0).expect("移动应成功");
        let tracks = list_playlist_tracks(&connection, id).expect("详情应成功");
        assert_eq!(tracks[0].bvid, "BV1PLC");
        assert_eq!(tracks[2].bvid, "BV1PLB");

        remove_track(&connection, id, "BV1PLA", 1).expect("移除应成功");
        let tracks = list_playlist_tracks(&connection, id).expect("详情应成功");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].bvid, "BV1PLC");
        assert_eq!(tracks[1].bvid, "BV1PLB");

        rename_playlist(&connection, id, "新名字").expect("重命名应成功");
        assert_eq!(
            get_playlist_name(&connection, id).expect("查询应成功").as_deref(),
            Some("新名字")
        );
        delete_playlist(&connection, id).expect("删除应成功");
        assert!(list_playlists(&connection).expect("列表应成功").is_empty());
    }
}
