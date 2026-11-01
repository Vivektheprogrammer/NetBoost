# 10 — API & Communication Flow: IPC Commands, Events, and Local REST Endpoints

---

## 1. Communication Architecture Overview

NetBoost utilizes two distinct communication boundaries:
1. **Internal Desktop IPC (Tauri IPC Channel)**: High-speed, in-memory JSON-RPC bridge connecting the React WebView2 frontend to the Rust core engine.
2. **Local Loopback REST API (`127.0.0.1:52140`)**: Lightweight HTTP/1.1 server connecting the companion Chromium browser extension to the NetBoost desktop core.

---

## 2. Complete Tauri IPC Commands Catalog

Every command below is declared with `#[tauri::command]` in `src-tauri/src/commands/` and registered in `lib.rs`.

---

### Category A: Network Telemetry & Diagnostics

#### 1. `get_realtime_metrics`
* **File**: `commands/network.rs`
* **Purpose**: Polls instantaneous adapter throughput, latency, jitter, packet loss, and health.
* **Input Parameters**: None (`State<'_, AppState>`).
* **Return Type**: `RealtimeMetrics`
  ```typescript
  interface RealtimeMetrics {
    downloadSpeedBps: number;
    uploadSpeedBps: number;
    totalBytesReceived: number;
    totalBytesSent: number;
    latencyMs: number;
    jitterMs: number;
    packetLossPercent: number;
    activeAdapter: NetworkAdapter | null;
    health: NetworkHealthReport;
    timestamp: number;
  }
  ```

#### 2. `get_network_adapters`
* **File**: `commands/network.rs`
* **Purpose**: Returns all physical, virtual, and loopback network adapters discovered on Windows via `GetIfTable2`.
* **Input Parameters**: None.
* **Return Type**: `Vec<NetworkAdapter>`

#### 3. `get_active_processes`
* **File**: `commands/network.rs`
* **Purpose**: Inspects active Windows TCP sockets and returns a list of processes, open remote endpoints, transfer rates, and streaming classifications.
* **Input Parameters**: None.
* **Return Type**: `Vec<ProcessNetworkReport>`

#### 4. `get_optimization_state`
* **File**: `commands/network.rs`
* **Purpose**: Queries the current status of the Bandwidth Coordinator.
* **Input Parameters**: None.
* **Return Type**: `OptimizationState` (enabled, mode, congestionDetected, pacingDelayMs, etc.).

#### 5. `set_optimization_mode`
* **File**: `commands/network.rs`
* **Purpose**: Switches active policy between `'streaming'`, `'balanced'`, and `'download'`.
* **Input Parameters**: `mode: OperatingMode`
* **Return Type**: `OptimizationState`

#### 6. `toggle_optimization`
* **File**: `commands/network.rs`
* **Purpose**: Globally enables or disables adaptive pacing.
* **Input Parameters**: `enabled: bool`
* **Return Type**: `OptimizationState`

---

### Category B: Download Accelerator Subsystem

#### 7. `start_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Probes target URL, pre-allocates `.part` file, partitions byte ranges, and spawns Tokio workers.
* **Input Parameters**:
  * `url: String`
  * `custom_filename: Option<String>`
  * `concurrency: Option<usize>` (e.g. 4, 6, 8, 12)
* **Return Type**: `Result<String, String>` (returns task ID string, e.g. `"dl-17304561234"`).

#### 8. `pause_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Signals active worker tasks to pause via `AtomicBool` and broadcast channel; updates status in SQLite to `'paused'`.
* **Input Parameters**: `id: String`
* **Return Type**: `Result<(), String>`

#### 9. `resume_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Re-probes URL to validate ETag and Last-Modified headers; spawns workers from last written byte offset.
* **Input Parameters**: `id: String`
* **Return Type**: `Result<(), String>`

#### 10. `cancel_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Stops workers, deletes the partial `.part` file from disk, and removes task from SQLite.
* **Input Parameters**: `id: String`
* **Return Type**: `Result<(), String>`

#### 11. `retry_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Resets error flags and re-attempts failed downloads.
* **Input Parameters**: `id: String`
* **Return Type**: `Result<(), String>`

#### 12. `get_downloads`
* **File**: `commands/downloads.rs`
* **Purpose**: Retrieves snapshot of all download tasks and segment states.
* **Input Parameters**: None.
* **Return Type**: `Vec<DownloadTask>`

#### 13. `open_downloaded_file`
* **File**: `commands/downloads.rs`
* **Purpose**: Launches Windows File Explorer with the downloaded file pre-selected (`explorer.exe /select,"<path>"`).
* **Input Parameters**: `path: String`
* **Return Type**: `Result<(), String>`

#### 14. `open_download_folder`
* **File**: `commands/downloads.rs`
* **Purpose**: Opens the parent folder containing the download.
* **Input Parameters**: `path: String`
* **Return Type**: `Result<(), String>`

#### 15. `open_external_url`
* **File**: `commands/downloads.rs`
* **Purpose**: Launches a URL in the user's default Windows web browser via `rundll32.exe url.dll,FileProtocolHandler`.
* **Input Parameters**: `url: String`
* **Return Type**: `Result<(), String>`

#### 16. `open_extension_folder`
* **File**: `commands/downloads.rs`
* **Purpose**: Locates the bundled `extension/` directory on the filesystem and opens it in Windows Explorer so the user can easily load it in Chrome/Edge via Developer Mode.
* **Input Parameters**: None.
* **Return Type**: `Result<(), String>`

#### 17. `resolve_intercepted_download`
* **File**: `commands/downloads.rs`
* **Purpose**: Delivers the user's decision from `QuickDownloadModal.tsx` to the waiting loopback server oneshot channel.
* **Input Parameters**:
  * `id: u64` (Chrome download ID)
  * `accept: bool` (`true` = accelerate with NetBoost, `false` = download in Chrome)
* **Return Type**: `()`

#### 18. `hide_window`
* **File**: `commands/downloads.rs`
* **Purpose**: Hides the main webview window to the system tray.
* **Input Parameters**: None.
* **Return Type**: `()`

---

### Category C: Network Benchmarking Engine

#### 19. `run_benchmark`
* **File**: `commands/benchmark.rs`
* **Purpose**: Conducts a controlled network benchmark (unloaded baseline probes + loaded download throughput).
* **Input Parameters**:
  * `scenario: String` (`"streaming_priority"`, `"balanced"`, `"download_priority"`, `"unoptimized"`, `"idle"`)
  * `target_url: Option<String>`
* **Return Type**: `Result<DbBenchmarkRecord, String>`

#### 20. `get_benchmark_history`
* **File**: `commands/benchmark.rs`
* **Purpose**: Returns the most recent 50 benchmark runs from SQLite.
* **Input Parameters**: None.
* **Return Type**: `Result<Vec<DbBenchmarkRecord>, String>`

#### 21. `clear_benchmark_history`
* **File**: `commands/benchmark.rs`
* **Purpose**: Deletes all records from the `benchmark_runs` SQLite table.
* **Input Parameters**: None.
* **Return Type**: `Result<(), String>`

#### 22. `get_benchmark_comparison`
* **File**: `commands/benchmark.rs`
* **Purpose**: Compares a benchmark run against a previous baseline, calculating latency delta and improvement percentages.
* **Input Parameters**: `current_id: String`
* **Return Type**: `Result<BenchmarkComparison, String>`

---

### Category D: Streaming Heuristics & App Configuration

#### 23. `get_streaming_config`
* **File**: `commands/streaming.rs`
* **Purpose**: Returns default streaming application list and user-configured custom application list.
* **Input Parameters**: None.
* **Return Type**: `StreamingConfig`

#### 24. `add_user_streaming_app`
* **File**: `commands/streaming.rs`
* **Purpose**: Adds an executable name (e.g. `"mygame.exe"`) to the streaming whitelist; persists to SQLite.
* **Input Parameters**: `app_name: String`
* **Return Type**: `Result<StreamingConfig, String>`

#### 25. `remove_user_streaming_app`
* **File**: `commands/streaming.rs`
* **Purpose**: Removes an executable from the custom streaming whitelist and SQLite.
* **Input Parameters**: `app_name: String`
* **Return Type**: `Result<StreamingConfig, String>`

---

### Category E: DNS Optimization & Adapter Restoration

#### 26. `benchmark_dns`
* **File**: `commands/dns.rs`
* **Purpose**: Probes Cloudflare, Google, Quad9, OpenDNS, and the current system resolver via UDP port 53.
* **Input Parameters**: None.
* **Return Type**: `Result<DnsBenchmarkReport, String>`

#### 27. `apply_dns_provider`
* **File**: `commands/dns.rs`
* **Purpose**: Automatically backs up existing adapter DNS configuration in SQLite and executes `netsh` to apply new static DNS servers.
* **Input Parameters**:
  * `interface_name: String` (e.g. `"Wi-Fi"`)
  * `primary_ip: String`
  * `secondary_ip: Option<String>`
* **Return Type**: `Result<(), String>`

#### 28. `restore_original_dns`
* **File**: `commands/dns.rs`
* **Purpose**: Reads original adapter configuration from SQLite and restores DHCP or original static IPs via `netsh`.
* **Input Parameters**: `interface_name: String`
* **Return Type**: `Result<(), String>`

#### 29. `get_dns_backup_status`
* **File**: `commands/dns.rs`
* **Purpose**: Checks if an active DNS backup exists for a given network interface.
* **Input Parameters**: `interface_name: String`
* **Return Type**: `Result<Option<DnsBackupRecord>, String>`

---

### Category F: Settings & System Logs

#### 30. `get_app_settings`
* **File**: `commands/settings.rs`
* **Purpose**: Reads settings from SQLite and queries the Windows Registry (`HKCU\Software\...\Run`) for auto-start status.
* **Input Parameters**: None.
* **Return Type**: `AppSettings`

#### 31. `save_app_settings`
* **File**: `commands/settings.rs`
* **Purpose**: Saves preferences to SQLite and updates the Windows Registry startup key accordingly.
* **Input Parameters**: `settings: AppSettings`
* **Return Type**: `Result<(), String>`

#### 32. `get_application_logs`
* **File**: `commands/settings.rs`
* **Purpose**: Reads the most recent 100 log lines from `%APPDATA%\NetBoost\logs\netboost.log`.
* **Input Parameters**: None.
* **Return Type**: `Vec<String>`

---

## 3. Asynchronous Tauri Events

### Event: `'netboost-download-detected'`
* **Emitter**: `src-tauri/src/downloads/interceptor.rs`
* **Listener**: `src/components/Downloads/QuickDownloadModal.tsx`
* **Payload**:
  ```typescript
  interface InterceptedDownload {
    id?: number;
    url: string;
    filename?: string;
    referrer?: string;
    cookies?: string;
    source: string; // "browser_extension"
  }
  ```
* **Behavior**: Triggered when a download click is captured by the browser extension. The React UI responds by displaying the Quick Download Modal immediately.

---

## 4. Local Loopback REST API (`127.0.0.1:52140`)

The local HTTP server runs continuously in a background Tokio task:

### Endpoint: `GET /status`
* **Purpose**: Health-check endpoint used by the extension to verify if NetBoost is running.
* **Response**:
  ```http
  HTTP/1.1 200 OK
  Content-Type: application/json

  {"status":"online","app":"NetBoost"}
  ```

### Endpoint: `POST /intercept`
* **Purpose**: Receives download metadata from the browser extension and holds the HTTP connection open until the user chooses an action in the UI.
* **Request Body**:
  ```json
  {
    "id": 142,
    "url": "https://example.com/large-archive.zip",
    "filename": "large-archive.zip",
    "referrer": "https://example.com/downloads",
    "cookies": "session=abc123xyz"
  }
  ```
* **Response**:
  ```json
  {
    "accelerate": true
  }
  ```
  *(or `{"accelerate": false}` if declined or timed out after 25s).*

### Endpoint: `OPTIONS *`
* **Purpose**: Handles Cross-Origin Resource Sharing (CORS) preflight requests from browser extension content scripts and service workers.
* **Response Headers**:
  * `Access-Control-Allow-Origin: *`
  * `Access-Control-Allow-Methods: POST, GET, OPTIONS`
  * `Access-Control-Allow-Headers: Content-Type`

