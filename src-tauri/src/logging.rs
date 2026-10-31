use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NetBoostLogger {
    log_file_path: PathBuf,
    lock: Mutex<()>,
}

impl NetBoostLogger {
    pub fn init() -> Self {
        let log_dir = if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data).join("NetBoost").join("logs")
        } else {
            PathBuf::from("logs")
        };

        let _ = create_dir_all(&log_dir);
        let log_file_path = log_dir.join("netboost.log");

        let logger = Self {
            log_file_path,
            lock: Mutex::new(()),
        };

        logger.log("INFO", "NetBoost networking engine initialized.");
        logger
    }

    pub fn log(&self, level: &str, message: &str) {
        let _guard = self.lock.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let line = format!("[{}] [{}] {}\n", now, level, message);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn get_recent_logs(&self, max_lines: usize) -> Vec<String> {
        let _guard = self.lock.lock().unwrap();
        if let Ok(content) = std::fs::read_to_string(&self.log_file_path) {
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].to_vec()
        } else {
            Vec::new()
        }
    }
}

