use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use rusqlite::Connection;
use walkdir::WalkDir;

use crate::db;
use crate::models::{LocalScanResult, LocalTrack};

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "aiff",
];

pub fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let normalized = normalized.trim_end_matches('/').to_string();
    if cfg!(windows) {
        normalized.to_lowercase()
    } else {
        normalized
    }
}

pub fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("m4a") | Some("aac") => "audio/mp4",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("ogg") | Some("opus") => "audio/ogg",
        Some("aiff") => "audio/aiff",
        _ => "application/octet-stream",
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

enum ScanOutcome {
    Added,
    Updated,
    Skipped,
}

pub fn scan_folder(
    conn: &Connection,
    folder_id: i64,
    folder_path: &Path,
    cover_root: &Path,
) -> Result<LocalScanResult, String> {
    let mut found: HashSet<String> = HashSet::new();
    let mut added = 0u64;
    let mut updated = 0u64;
    let mut skipped = 0u64;
    let mut failed = 0u64;

    for entry in WalkDir::new(folder_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            continue;
        }
        let path_str = path.to_string_lossy().into_owned();
        found.insert(normalize_path(&path_str));
        match upsert_local_file(conn, path, folder_id, cover_root) {
            Ok(ScanOutcome::Added) => added += 1,
            Ok(ScanOutcome::Updated) => updated += 1,
            Ok(ScanOutcome::Skipped) => skipped += 1,
            Err(_) => failed += 1,
        }
    }

    let removed = db::local::delete_tracks_not_in(conn, folder_id, &found)
        .map_err(|error| error.to_string())?;

    let mut result = db::local::scan_outcome(
        folder_id,
        &folder_path.to_string_lossy(),
        added,
        updated,
        removed,
        skipped,
    );
    result.skipped += failed;
    Ok(result)
}

fn upsert_local_file(
    conn: &Connection,
    path: &Path,
    folder_id: i64,
    cover_root: &Path,
) -> Result<ScanOutcome, String> {
    let path_str = normalize_path(&path.to_string_lossy());
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let size = metadata.len();
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if let Ok(Some(existing)) = db::local::get_track_by_path(conn, &path_str) {
        if existing.size == size
            && existing.modified_at == modified_at
            && existing.cover_path.is_some()
        {
            return Ok(ScanOutcome::Skipped);
        }
    }
    let is_new = db::local::get_track_by_path(conn, &path_str)
        .map_err(|error| error.to_string())?
        .is_none();

    let file_stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("未知曲目")
        .to_string();
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    let tagged = lofty::probe::Probe::open(path)
        .and_then(|probe| probe.read())
        .ok();
    let (title, artist, album, duration, codec) = match &tagged {
        Some(tagged) => {
            let tag = tagged.primary_tag();
            let title = tag
                .and_then(|t| t.title())
                .map(|value| value.into_owned())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| file_stem.clone());
            let artist = tag
                .and_then(|t| t.artist())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            let album = tag
                .and_then(|t| t.album())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            let duration = tagged.properties().duration().as_secs();
            let codec = format!("{:?}", tagged.file_type()).to_lowercase();
            (title, artist, album, duration, codec)
        }
        None => (file_stem, String::new(), String::new(), 0, ext),
    };

    let track = LocalTrack {
        id: 0,
        path: path_str.clone(),
        folder_id: Some(folder_id),
        title,
        artist,
        album,
        duration,
        codec,
        size,
        modified_at,
        cover_path: None,
        added_at: now_secs(),
        last_played_at: None,
        play_count: 0,
    };
    let id = db::local::upsert_track(conn, &track, now_secs()).map_err(|error| error.to_string())?;

    let mut cover_written = false;
    if let Some(picture) = tagged.as_ref().and_then(|t| t.primary_tag()).and_then(|t| t.pictures().first()) {
        let data = picture.data();
        if !data.is_empty() {
            let mime = picture
                .mime_type()
                .map(|mime| format!("{mime:?}"))
                .unwrap_or_default();
            let ext = if mime.contains("png") { "png" } else { "jpg" };
            let cover_path = cover_root.join(format!("{id}.{ext}"));
            if let Ok(()) = fs::create_dir_all(cover_root)
                .and_then(|()| fs::write(&cover_path, data))
            {
                let _ = db::local::set_cover_path(conn, id, &cover_path.to_string_lossy());
                cover_written = true;
            }
        }
    }
    if !cover_written {
        let _ = db::local::set_cover_path(conn, id, "");
    }

    Ok(if is_new { ScanOutcome::Added } else { ScanOutcome::Updated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn minimal_wav(path: &Path) {
        let samples: Vec<u8> = (0..800u16).map(|i| (i % 128) as u8).collect();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&samples);
        fs::write(path, bytes).expect("wav 应可写入");
    }

    #[test]
    fn scan_detects_add_and_remove() {
        let dir = std::env::temp_dir().join(format!(
            "sentune-local-scan-{}",
            std::process::id()
        ));
        let music = dir.join("music");
        fs::create_dir_all(&music).expect("临时目录应可创建");
        minimal_wav(&music.join("a.wav"));

        let conn = Connection::open_in_memory().expect("内存数据库应可创建");
        conn.execute_batch(crate::db::SCHEMA).expect("建表应成功");
        let folder_id = db::local::add_folder(&conn, &music.to_string_lossy(), 1000)
            .expect("文件夹应可添加");
        let result = scan_folder(&conn, folder_id, &music, &dir.join("covers"))
            .expect("扫描应成功");
        assert_eq!(result.added, 1);
        let tracks = db::local::list_tracks(&conn, None, None).expect("列表应成功");
        assert_eq!(tracks.len(), 1);

        fs::remove_file(music.join("a.wav")).expect("测试文件应可删除");
        let result = scan_folder(&conn, folder_id, &music, &dir.join("covers"))
            .expect("二次扫描应成功");
        assert_eq!(result.removed, 1);
        assert!(db::local::list_tracks(&conn, None, None).expect("列表应成功").is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn normalize_path_unifies_separators() {
        assert_eq!(normalize_path("C:\\Music\\a.mp3"), normalize_path("c:/music/a.mp3"));
        assert!(normalize_path("C:/Music/").ends_with("c:/music"));
    }
}
