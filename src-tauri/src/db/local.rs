use std::collections::HashSet;
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{LocalFolder, LocalScanResult, LocalTrack};

pub fn add_folder(conn: &Connection, path: &str, now: i64) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO local_folders (path, added_at) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET path = excluded.path",
        params![path, now],
    )?;
    let id = conn.query_row(
        "SELECT id FROM local_folders WHERE path = ?1",
        params![path],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn list_folders(conn: &Connection) -> rusqlite::Result<Vec<LocalFolder>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.path, f.added_at,
                (SELECT COUNT(*) FROM local_tracks t WHERE t.folder_id = f.id)
         FROM local_folders f ORDER BY f.added_at DESC, f.path ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(LocalFolder {
            id: row.get(0)?,
            path: row.get(1)?,
            added_at: row.get(2)?,
            track_count: row.get::<_, i64>(3)? as u64,
        })
    })?;
    rows.collect()
}

pub fn get_folder(conn: &Connection, id: i64) -> rusqlite::Result<Option<LocalFolder>> {
    conn.query_row(
        "SELECT f.id, f.path, f.added_at,
                (SELECT COUNT(*) FROM local_tracks t WHERE t.folder_id = f.id)
         FROM local_folders f WHERE f.id = ?1",
        params![id],
        |row| {
            Ok(LocalFolder {
                id: row.get(0)?,
                path: row.get(1)?,
                added_at: row.get(2)?,
                track_count: row.get::<_, i64>(3)? as u64,
            })
        },
    )
    .optional()
}

pub fn delete_folder(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM local_favorites WHERE track_id IN
         (SELECT id FROM local_tracks WHERE folder_id = ?1)",
        params![id],
    )?;
    conn.execute(
        "DELETE FROM local_history WHERE track_id IN
         (SELECT id FROM local_tracks WHERE folder_id = ?1)",
        params![id],
    )?;
    conn.execute("DELETE FROM local_tracks WHERE folder_id = ?1", params![id])?;
    conn.execute("DELETE FROM local_folders WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn upsert_track(
    conn: &Connection,
    track: &LocalTrack,
    now: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO local_tracks
           (path, folder_id, title, artist, album, duration, codec, size,
            modified_at, cover_path, added_at, last_played_at, play_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(path) DO UPDATE SET
           folder_id = excluded.folder_id,
           title = excluded.title,
           artist = excluded.artist,
           album = excluded.album,
           duration = excluded.duration,
           codec = excluded.codec,
           size = excluded.size,
           modified_at = excluded.modified_at,
           cover_path = excluded.cover_path,
           added_at = excluded.added_at",
        params![
            track.path,
            track.folder_id,
            track.title,
            track.artist,
            track.album,
            track.duration,
            track.codec,
            track.size,
            track.modified_at,
            track.cover_path,
            now,
            track.last_played_at,
            track.play_count
        ],
    )?;
    conn.query_row(
        "SELECT id FROM local_tracks WHERE path = ?1",
        params![track.path],
        |row| row.get(0),
    )
}

pub fn get_track_by_path(conn: &Connection, path: &str) -> rusqlite::Result<Option<LocalTrack>> {
    conn.query_row(
        "SELECT id, path, folder_id, title, artist, album, duration, codec, size,
                modified_at, cover_path, added_at, last_played_at, play_count
         FROM local_tracks WHERE path = ?1",
        params![path],
        row_to_track,
    )
    .optional()
}

pub fn set_cover_path(conn: &Connection, id: i64, cover_path: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE local_tracks SET cover_path = ?1 WHERE id = ?2",
        params![cover_path, id],
    )?;
    Ok(())
}

pub fn list_tracks(
    conn: &Connection,
    query: Option<&str>,
    folder_id: Option<i64>,
) -> rusqlite::Result<Vec<LocalTrack>> {
    const COLS: &str = "id, path, folder_id, title, artist, album, duration, codec, size,
                modified_at, cover_path, added_at, last_played_at, play_count";
    let query = query.map(str::trim).filter(|q| !q.is_empty());
    let pattern = query.map(|q| format!("%{q}%"));
    let mut stmt;
    let rows: Vec<LocalTrack> = match (folder_id, pattern.as_deref()) {
        (Some(folder_id), Some(pattern)) => {
            stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM local_tracks
                 WHERE folder_id = ?1 AND (title LIKE ?2 OR artist LIKE ?2 OR album LIKE ?2)
                 ORDER BY artist ASC, album ASC, title ASC"
            ))?;
            stmt.query_map(params![folder_id, pattern], |row| row_to_track(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        }
        (Some(folder_id), None) => {
            stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM local_tracks
                 WHERE folder_id = ?1
                 ORDER BY artist ASC, album ASC, title ASC"
            ))?;
            stmt.query_map(params![folder_id], |row| row_to_track(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        }
        (None, Some(pattern)) => {
            stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM local_tracks
                 WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1
                 ORDER BY artist ASC, album ASC, title ASC"
            ))?;
            stmt.query_map(params![pattern], |row| row_to_track(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        }
        (None, None) => {
            stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM local_tracks
                 ORDER BY artist ASC, album ASC, title ASC"
            ))?;
            stmt.query_map([], |row| row_to_track(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        }
    };
    Ok(rows)
}

pub fn delete_track(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM local_favorites WHERE track_id = ?1", params![id])?;
    conn.execute("DELETE FROM local_history WHERE track_id = ?1", params![id])?;
    conn.execute("DELETE FROM local_tracks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list_paths_by_folder(conn: &Connection, folder_id: i64) -> rusqlite::Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT path FROM local_tracks WHERE folder_id = ?1")?;
    let rows = stmt.query_map(params![folder_id], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn delete_tracks_not_in(
    conn: &Connection,
    folder_id: i64,
    keep: &HashSet<String>,
) -> rusqlite::Result<u64> {
    let existing = list_paths_by_folder(conn, folder_id)?;
    let mut removed = 0u64;
    for path in existing {
        if !keep.contains(&path) {
            let id: i64 = conn.query_row(
                "SELECT id FROM local_tracks WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )?;
            delete_track(conn, id)?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn add_favorite(conn: &Connection, track_id: i64, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO local_favorites (track_id, created_at) VALUES (?1, ?2)",
        params![track_id, now],
    )?;
    Ok(())
}

pub fn remove_favorite(conn: &Connection, track_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM local_favorites WHERE track_id = ?1", params![track_id])?;
    Ok(())
}

pub fn list_favorites(conn: &Connection) -> rusqlite::Result<Vec<LocalTrack>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.path, t.folder_id, t.title, t.artist, t.album, t.duration,
                t.codec, t.size, t.modified_at, t.cover_path, t.added_at,
                t.last_played_at, t.play_count
         FROM local_favorites f
         JOIN local_tracks t ON t.id = f.track_id
         ORDER BY f.created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| row_to_track(row))?;
    rows.collect()
}

pub fn add_history(conn: &Connection, track_id: i64, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO local_history (track_id, played_at) VALUES (?1, ?2)
         ON CONFLICT(track_id) DO UPDATE SET played_at = excluded.played_at",
        params![track_id, now],
    )?;
    conn.execute(
        "UPDATE local_tracks SET last_played_at = ?1, play_count = play_count + 1 WHERE id = ?2",
        params![now, track_id],
    )?;
    Ok(())
}

pub fn list_history(conn: &Connection) -> rusqlite::Result<Vec<LocalTrack>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.path, t.folder_id, t.title, t.artist, t.album, t.duration,
                t.codec, t.size, t.modified_at, t.cover_path, t.added_at,
                t.last_played_at, t.play_count
         FROM local_history h
         JOIN local_tracks t ON t.id = h.track_id
         ORDER BY h.played_at DESC",
    )?;
    let rows = stmt.query_map([], |row| row_to_track(row))?;
    rows.collect()
}

pub fn clear_history(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM local_history", [])?;
    Ok(())
}

pub fn folder_paths(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM local_folders")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn get_cover_path(conn: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT cover_path FROM local_tracks WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
}

pub fn is_allowed_path(conn: &Connection, path: &str) -> bool {
    let Ok(folders) = folder_paths(conn) else {
        return false;
    };
    let normalized = crate::local::normalize_path(path);
    folders
        .iter()
        .map(|folder| crate::local::normalize_path(folder))
        .any(|folder| {
            normalized == folder || normalized.starts_with(&format!("{folder}/"))
        })
}

pub fn scan_outcome(folder_id: i64, path: &str, added: u64, updated: u64, removed: u64, skipped: u64) -> LocalScanResult {
    LocalScanResult {
        folder_id,
        path: path.to_string(),
        added,
        updated,
        removed,
        skipped,
    }
}

fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<LocalTrack> {
    Ok(LocalTrack {
        id: row.get(0)?,
        path: row.get(1)?,
        folder_id: row.get(2)?,
        title: row.get(3)?,
        artist: row.get(4)?,
        album: row.get(5)?,
        duration: row.get::<_, i64>(6)? as u64,
        codec: row.get(7)?,
        size: row.get::<_, i64>(8)? as u64,
        modified_at: row.get(9)?,
        cover_path: row.get(10)?,
        added_at: row.get(11)?,
        last_played_at: row.get(12)?,
        play_count: row.get::<_, i64>(13)? as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("内存数据库应可创建");
        conn.execute_batch(crate::db::SCHEMA).expect("建表应成功");
        conn
    }

    #[test]
    fn local_track_upsert_list_delete() {
        let conn = test_conn();
        let folder_id = crate::db::local::add_folder(&conn, "C:/Music", 1000)
            .expect("文件夹应可添加");
        let track = LocalTrack {
            id: 0,
            path: "C:/Music/song.mp3".to_string(),
            folder_id: Some(folder_id),
            title: "歌".to_string(),
            artist: "歌手".to_string(),
            album: "专辑".to_string(),
            duration: 120,
            codec: "mp3".to_string(),
            size: 1024,
            modified_at: 2000,
            cover_path: None,
            added_at: 1000,
            last_played_at: None,
            play_count: 0,
        };
        let id = crate::db::local::upsert_track(&conn, &track, 1000)
            .expect("曲目应可写入");
        assert!(id > 0);
        let tracks = crate::db::local::list_tracks(&conn, None, None).expect("列表应成功");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "歌");
        let found = crate::db::local::get_track_by_path(&conn, "C:/Music/song.mp3")
            .expect("查询应成功");
        assert!(found.is_some());
        crate::db::local::delete_track(&conn, id).expect("删除应成功");
        assert!(
            crate::db::local::list_tracks(&conn, None, None)
                .expect("列表应成功")
                .is_empty()
        );
    }

    #[test]
    fn path_validation_blocks_outside_files() {
        let conn = test_conn();
        crate::db::local::add_folder(&conn, "C:/Music", 1000).expect("文件夹应可添加");
        assert!(crate::db::local::is_allowed_path(&conn, "C:/Music/song.mp3"));
        assert!(crate::db::local::is_allowed_path(&conn, "C:/Music/sub/song.flac"));
        assert!(!crate::db::local::is_allowed_path(&conn, "C:/Other/song.mp3"));
        assert!(!crate::db::local::is_allowed_path(&conn, "C:/MusicBox/song.mp3"));
    }
}
