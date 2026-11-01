# 09 — Database and Data Management: SQLite Persistence & Crash Recovery

---

## 1. Data Layer Overview

NetBoost utilizes an embedded, serverless relational database engine: **SQLite (v3.x)**, integrated via the Rust crate `rusqlite` (v0.32) compiled with the `bundled` feature.

### Architectural Highlights
* **Zero External Dependencies**: SQLite is compiled directly into the `NetBoost.exe` binary from C source code. End users do not need to install database servers, drivers, or ODBC connectors.
* **Persistent Location**: Stored in the user's roaming application directory: `%APPDATA%\NetBoost\netboost.db`.
* **In-Memory Fallback**: If file system permissions prevent writing to disk, `AppState::new()` falls back gracefully to `Database::in_memory()`.
* **Thread Safety**: Access to the SQLite `Connection` is synchronized using a `std::sync::Mutex<Connection>`, guaranteeing ACID transaction integrity and preventing multi-threaded database corruption.

---

## 2. Database Schema Definition

All tables and indexes are initialized automatically during startup via `init_schema()` in `src-tauri/src/storage/db.rs` using `conn.execute_batch(...)`:

```sql
-- 1. Download Task History & State Table
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

-- 2. Network Benchmark Results Table
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

-- 3. User-Configured Streaming Application Whitelist
CREATE TABLE IF NOT EXISTS user_streaming_apps (
    app_name TEXT PRIMARY KEY,
    added_at INTEGER NOT NULL
);

-- 4. Windows Network Adapter DNS Backup Table
CREATE TABLE IF NOT EXISTS dns_backups (
    interface_name TEXT PRIMARY KEY,
    is_dhcp INTEGER NOT NULL,
    primary_dns TEXT,
    secondary_dns TEXT,
    backed_up_at INTEGER NOT NULL
);

-- 5. Key-Value Application Settings Table
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

---

## 3. Table Details & Field Analysis

### 3.1 Table: `downloads`
Stores active, paused, completed, and failed file download tasks.

| Column Name | SQLite Type | Rust Type | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `String` | Primary key generated as `dl-{unix_millis}`. |
| `url` | `TEXT` | `String` | Source HTTP/HTTPS download URL. |
| `filename` | `TEXT` | `String` | File name extracted from headers or URL path. |
| `destination_path` | `TEXT` | `String` | Absolute path on disk (e.g. `C:\Users\user\Downloads\file.zip`). |
| `total_bytes` | `INTEGER` | `u64` | Total payload size in bytes from `Content-Range` or `Content-Length`. |
| `downloaded_bytes`| `INTEGER` | `u64` | Total bytes successfully written to disk. |
| `status` | `TEXT` | `String` | State: `'queued'`, `'downloading'`, `'paused'`, `'completed'`, `'failed'`. |
| `etag` | `TEXT` | `Option<String>` | HTTP `ETag` header used to verify remote file hasn't changed upon resume. |
| `last_modified` | `TEXT` | `Option<String>` | HTTP `Last-Modified` header for cache/resume validation. |
| `checksum_sha256` | `TEXT` | `Option<String>` | Hex-encoded SHA-256 cryptographic hash computed upon completion. |
| `error_message` | `TEXT` | `Option<String>` | Human-readable error description if download failed. |
| `concurrency` | `INTEGER` | `u32` | Number of parallel worker channels allocated (1 to 16). |
| `created_at` | `INTEGER` | `u64` | Millisecond epoch timestamp when task was created. |
| `completed_at` | `INTEGER` | `Option<u64>` | Millisecond epoch timestamp when task finished. |

### 3.2 Table: `benchmark_runs`
Records controlled network benchmarks comparing idle vs. loaded performance.

| Column Name | SQLite Type | Rust Type | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `String` | Primary key generated as `bm-{unix_millis}`. |
| `timestamp` | `INTEGER` | `u64` | Millisecond epoch timestamp of benchmark execution. |
| `mode` | `TEXT` | `String` | Operating mode active during test (`streaming`, `balanced`, `download`). |
| `unloaded_latency_ms`| `REAL` | `f64` | Median baseline RTT ping (ms) measured during idle phase. |
| `loaded_latency_ms` | `REAL` | `f64` | Median RTT ping (ms) measured while saturated with active download load. |
| `jitter_ms` | `REAL` | `f64` | RFC 3550 packet interarrival jitter variance (ms). |
| `packet_loss_percent`| `REAL` | `f64` | Percentage of dropped connect probes during benchmark. |
| `download_speed_bps`| `REAL` | `f64` | Raw download throughput achieved in bits per second. |
| `completion_time_secs`| `REAL` | `f64` | Duration of the benchmark test run. |
| `optimization_enabled`| `INTEGER` | `bool` | `1` if NetBoost pacing was active, `0` if unoptimized. |
| `scenario` | `TEXT` | `String` | Benchmark scenario label (e.g. `"Streaming Priority Baseline"`). |
| `notes` | `TEXT` | `Option<String>` | Diagnostic summary detailing bufferbloat delta. |

### 3.3 Table: `dns_backups`
Guarantees 1-click restoration of network adapter DNS configuration.

| Column Name | SQLite Type | Rust Type | Description |
| :--- | :--- | :--- | :--- |
| `interface_name` | `TEXT` | `String` | Primary key (e.g. `"Wi-Fi"`, `"Ethernet"`). |
| `is_dhcp` | `INTEGER` | `bool` | `1` if adapter was configured via automatic DHCP, `0` if static. |
| `primary_dns` | `TEXT` | `Option<String>` | Original primary IPv4 resolver (e.g. `"192.168.1.1"`). |
| `secondary_dns` | `TEXT` | `Option<String>` | Original secondary IPv4 resolver if previously configured. |
| `backed_up_at` | `INTEGER` | `u64` | Millisecond epoch timestamp when backup was created. |

---

## 4. CRUD Operations & Rust Code Mapping

### 4.1 Idempotent Upsert for Downloads
When updating download task progress, NetBoost uses SQLite's `ON CONFLICT` clause to prevent race conditions:

```rust
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
            rec.id, rec.url, rec.filename, rec.destination_path,
            rec.total_bytes as i64, rec.downloaded_bytes as i64,
            rec.status, rec.etag, rec.last_modified,
            rec.checksum_sha256, rec.error_message,
            rec.concurrency as i64, rec.created_at as i64,
            rec.completed_at.map(|v| v as i64),
        ],
    )?;
    Ok(())
}
```

### 4.2 Startup Crash Recovery
To handle sudden computer power loss or OS termination, `recover_interrupted_downloads()` is invoked at application startup:

```rust
pub fn recover_interrupted_downloads(&self) -> Result<usize> {
    let conn = self.conn.lock().unwrap();
    let affected = conn.execute(
        "UPDATE downloads SET status = 'paused' WHERE status = 'downloading'",
        [],
    )?;
    Ok(affected)
}
```
* **Execution**: Before any worker tasks are spawned, this query transitions all tasks that were in an unfinalized `downloading` state to `paused`.
* **Integrity Guarantee**: This eliminates invalid download states on boot and allows the user to resume downloads cleanly with ETag validation.

### 4.3 Safe Key-Value App Settings
User preferences (such as minimize-to-tray or default mode) are persisted using key-value pairs in `app_settings`:

```rust
pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}
```

