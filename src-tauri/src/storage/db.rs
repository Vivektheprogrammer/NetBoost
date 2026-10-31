use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct DbDownloadRecord {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub destination_path: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub status: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub checksum_sha256: Option<String>,
    pub error_message: Option<String>,
    pub concurrency: u32,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbBenchmarkRecord {
    pub id: String,
    pub timestamp: u64,
    pub mode: String,
    #[serde(rename = "unloadedLatencyMs")]
    pub unloaded_latency_ms: f64,
    #[serde(rename = "loadedLatencyMs")]
    pub loaded_latency_ms: f64,
    #[serde(rename = "jitterMs")]
    pub jitter_ms: f64,
    #[serde(rename = "packetLossPercent")]
    pub packet_loss_percent: f64,
    #[serde(rename = "downloadSpeedBps")]
    pub download_speed_bps: f64,
    #[serde(rename = "completionTimeSecs")]
    pub completion_time_secs: f64,
    #[serde(rename = "optimizationEnabled")]
    pub optimization_enabled: bool,
    pub scenario: String,
    pub notes: Option<String>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS downloads (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                filename TEXT NOT NULL,
                destination_path TEXT NOT NULL,
                total_bytes INTEGER NOT NULL,
                downloaded_bytes INTEGER NOT NULL,
                status TEXT NOT NULL,
                etag TEXT,
                last_modified TEXT,
                checksum_sha256 TEXT,
                error_message TEXT,
                concurrency INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                completed_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads (created_at DESC);

            CREATE TABLE IF NOT EXISTS benchmark_runs (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                mode TEXT NOT NULL,
                unloaded_latency_ms REAL NOT NULL,
                loaded_latency_ms REAL NOT NULL,
                jitter_ms REAL NOT NULL,
                packet_loss_percent REAL NOT NULL,
                download_speed_bps REAL NOT NULL,
                completion_time_secs REAL NOT NULL,
                optimization_enabled INTEGER NOT NULL,
                scenario TEXT NOT NULL,
                notes TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_benchmarks_ts ON benchmark_runs (timestamp DESC);

            CREATE TABLE IF NOT EXISTS user_streaming_apps (
                app_name TEXT PRIMARY KEY,
                added_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dns_backups (
                interface_name TEXT PRIMARY KEY,
                is_dhcp INTEGER NOT NULL,
                primary_dns TEXT,
                secondary_dns TEXT,
                backed_up_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn save_download(&self, rec: &DbDownloadRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "
            INSERT INTO downloads (
                id, url, filename, destination_path, total_bytes, downloaded_bytes,
                status, etag, last_modified, checksum_sha256, error_message, concurrency, created_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO UPDATE SET
                downloaded_bytes=excluded.downloaded_bytes,
                status=excluded.status,
                etag=excluded.etag,
                last_modified=excluded.last_modified,
                checksum_sha256=excluded.checksum_sha256,
                error_message=excluded.error_message,
                completed_at=excluded.completed_at;
            ",
            params![
                rec.id,
                rec.url,
                rec.filename,
                rec.destination_path,
                rec.total_bytes as i64,
                rec.downloaded_bytes as i64,
                rec.status,
                rec.etag,
                rec.last_modified,
                rec.checksum_sha256,
                rec.error_message,
                rec.concurrency as i64,
                rec.created_at as i64,
                rec.completed_at.map(|v| v as i64),
            ],
        )?;
        Ok(())
    }

    pub fn get_all_downloads(&self) -> Result<Vec<DbDownloadRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, filename, destination_path, total_bytes, downloaded_bytes,
                    status, etag, last_modified, checksum_sha256, error_message, concurrency, created_at, completed_at
             FROM downloads ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let total: i64 = row.get(4)?;
            let dl: i64 = row.get(5)?;
            let conc: i64 = row.get(11)?;
            let created: i64 = row.get(12)?;
            let comp: Option<i64> = row.get(13)?;

            Ok(DbDownloadRecord {
                id: row.get(0)?,
                url: row.get(1)?,
                filename: row.get(2)?,
                destination_path: row.get(3)?,
                total_bytes: total as u64,
                downloaded_bytes: dl as u64,
                status: row.get(6)?,
                etag: row.get(7)?,
                last_modified: row.get(8)?,
                checksum_sha256: row.get(9)?,
                error_message: row.get(10)?,
                concurrency: conc as u32,
                created_at: created as u64,
                completed_at: comp.map(|v| v as u64),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_download(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Benchmark History Operations ---
    pub fn save_benchmark_run(&self, rec: &DbBenchmarkRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "
            INSERT INTO benchmark_runs (
                id, timestamp, mode, unloaded_latency_ms, loaded_latency_ms,
                jitter_ms, packet_loss_percent, download_speed_bps, completion_time_secs,
                optimization_enabled, scenario, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params![
                rec.id,
                rec.timestamp as i64,
                rec.mode,
                rec.unloaded_latency_ms,
                rec.loaded_latency_ms,
                rec.jitter_ms,
                rec.packet_loss_percent,
                rec.download_speed_bps,
                rec.completion_time_secs,
                if rec.optimization_enabled { 1i64 } else { 0i64 },
                rec.scenario,
                rec.notes,
            ],
        )?;
        Ok(())
    }

    pub fn get_benchmark_history(&self) -> Result<Vec<DbBenchmarkRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, mode, unloaded_latency_ms, loaded_latency_ms,
                    jitter_ms, packet_loss_percent, download_speed_bps, completion_time_secs,
                    optimization_enabled, scenario, notes
             FROM benchmark_runs ORDER BY timestamp DESC LIMIT 50"
        )?;

        let rows = stmt.query_map([], |row| {
            let ts: i64 = row.get(1)?;
            let opt: i64 = row.get(9)?;

            Ok(DbBenchmarkRecord {
                id: row.get(0)?,
                timestamp: ts as u64,
                mode: row.get(2)?,
                unloaded_latency_ms: row.get(3)?,
                loaded_latency_ms: row.get(4)?,
                jitter_ms: row.get(5)?,
                packet_loss_percent: row.get(6)?,
                download_speed_bps: row.get(7)?,
                completion_time_secs: row.get(8)?,
                optimization_enabled: opt != 0,
                scenario: row.get(10)?,
                notes: row.get(11)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn clear_benchmark_history(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM benchmark_runs", [])?;
        Ok(())
    }

    // --- User Streaming Applications Configuration ---
    pub fn add_streaming_app(&self, app_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        conn.execute(
            "INSERT OR IGNORE INTO user_streaming_apps (app_name, added_at) VALUES (?1, ?2)",
            params![app_name.to_lowercase(), now],
        )?;
        Ok(())
    }

    pub fn remove_streaming_app(&self, app_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM user_streaming_apps WHERE app_name = ?1",
            params![app_name.to_lowercase()],
        )?;
        Ok(())
    }

    pub fn get_streaming_apps(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT app_name FROM user_streaming_apps ORDER BY app_name ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // --- DNS Backup & Restoration Operations ---
    pub fn save_dns_backup(&self, rec: &DnsBackupRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO dns_backups (interface_name, is_dhcp, primary_dns, secondary_dns, backed_up_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(interface_name) DO UPDATE SET
                is_dhcp=excluded.is_dhcp,
                primary_dns=excluded.primary_dns,
                secondary_dns=excluded.secondary_dns,
                backed_up_at=excluded.backed_up_at",
            params![
                rec.interface_name,
                if rec.is_dhcp { 1 } else { 0 },
                rec.primary_dns,
                rec.secondary_dns,
                rec.backed_up_at as i64
            ],
        )?;
        Ok(())
    }

    pub fn get_dns_backup(&self, interface_name: &str) -> Result<Option<DnsBackupRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT interface_name, is_dhcp, primary_dns, secondary_dns, backed_up_at
             FROM dns_backups WHERE interface_name = ?1",
        )?;
        let mut rows = stmt.query(params![interface_name])?;
        if let Some(row) = rows.next()? {
            let is_dhcp: i64 = row.get(1)?;
            let ts: i64 = row.get(4)?;
            Ok(Some(DnsBackupRecord {
                interface_name: row.get(0)?,
                is_dhcp: is_dhcp != 0,
                primary_dns: row.get(2)?,
                secondary_dns: row.get(3)?,
                backed_up_at: ts as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_dns_backup(&self, interface_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM dns_backups WHERE interface_name = ?1", params![interface_name])?;
        Ok(())
    }

    // --- Application Settings Operations ---
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    // --- Startup Crash Recovery ---
    pub fn recover_interrupted_downloads(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE downloads SET status = 'paused' WHERE status = 'downloading'",
            [],
        )?;
        Ok(affected)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsBackupRecord {
    #[serde(rename = "interfaceName")]
    pub interface_name: String,
    #[serde(rename = "isDhcp")]
    pub is_dhcp: bool,
    #[serde(rename = "primaryDns")]
    pub primary_dns: Option<String>,
    #[serde(rename = "secondaryDns")]
    pub secondary_dns: Option<String>,
    #[serde(rename = "backedUpAt")]
    pub backed_up_at: u64,
}
