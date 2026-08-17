use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_FILE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// 初始化日志并滚动保留最近 3 份（sentune.log / .1.log / .2.log）。
pub fn init(dir: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;
    let _ = fs::remove_file(dir.join("sentune.2.log"));
    let _ = fs::rename(
        dir.join("sentune.1.log"),
        dir.join("sentune.2.log"),
    );
    let _ = fs::rename(dir.join("sentune.log"), dir.join("sentune.1.log"));
    let path = dir.join("sentune.log");
    *LOG_FILE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(path);
    Ok(())
}

fn write(level: &str, message: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{ts}] [{level}] {message}\n");
    let path = LOG_FILE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if let Some(path) = path {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
    eprintln!("{line}");
}

pub fn info(message: &str) {
    write("INFO", message);
}

pub fn error(message: &str) {
    write("ERROR", message);
}
