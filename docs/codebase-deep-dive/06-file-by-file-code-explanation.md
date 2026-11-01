# 06 — File-by-File Code Explanation: Comprehensive Source Code Audit

---

## Guide to this Document

This document is the definitive line-by-line and component-by-component architectural reference for every source file in the NetBoost repository. For each file, we analyze:
1. **File Overview**: Purpose, architectural layer, upstream and downstream dependencies.
2. **Imports & Dependencies**: Exactly why each external crate or module is imported.
3. **Internal Mechanics & Line-by-Line Analysis**: How memory, pointers, channels, and locks behave.
4. **Functions & Data Structures**: Parameters, types, return values, side effects, and failure modes.
5. **Concrete Execution Trace**: Step-by-step data transformations during execution.

---

# PART I: RUST SYSTEMS BACKEND (`src-tauri/src/`)

---

## 1. `src-tauri/src/main.rs`

### File Overview
* **Path**: `src-tauri/src/main.rs`
* **Purpose**: Native Windows process entry point for the compiled executable.
* **Component**: Core Bootstrap.
* **Dependencies**: Calls `netboost_lib::run()`.

### Code Walkthrough & Analysis
```rust
// Prevents additional console window on Windows, DO NOT REMOVE!!
#![windows_subsystem = "windows"]
```
* **`#![windows_subsystem = "windows"]`**: An inner attribute (indicated by `#!`) applied to the entire crate root. On Windows, standard Rust executables default to the console subsystem, meaning Windows creates a command prompt window when launched. This directive instructs the linker to set the PE header subsystem field to `IMAGE_SUBSYSTEM_WINDOWS_GUI`, launching the process silently without a black CMD window.

```rust
fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("CRASH PANIC: {}", info);
        let _ = std::fs::write("C:\\NetBoost\\panic.log", &msg);
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            let _ = std::fs::write(format!("{}\\NetBoost\\logs\\crash.log", appdata), &msg);
        }
    }));

    netboost_lib::run();
}
```
* **`std::panic::set_hook(...)`**: Installs a custom panic handler closure. The closure receives `&PanicHookInfo`, which encapsulates the panic payload (error message) and file/line source location.
* **`Box::new(|info| { ... })`**: Allocates the closure on the heap because `set_hook` expects a trait object `Box<dyn Fn(&PanicHookInfo) + 'static + Sync + Send>`.
* **Crash Logging**: Attempts to write the panic message to `C:\NetBoost\panic.log` first, then inspects the `APPDATA` environment variable using `std::env::var` to write to `%APPDATA%\NetBoost\logs\crash.log`. The `let _ = ...` pattern intentionally suppresses write errors so that a failure to write to a read-only directory does not trigger a recursive panic during crash handling.
* **`netboost_lib::run()`**: Transfers execution to the library entry point in `lib.rs`.

---

## 2. `src-tauri/src/lib.rs`

### File Overview
* **Path**: `src-tauri/src/lib.rs`
* **Purpose**: Central library coordinator; configures Tauri runtime, registers 22 IPC commands, builds system tray, and starts background loopback listener.
* **Component**: Application Core & IPC Router.
* **Dependencies**: Imported by `main.rs`; imports all submodules (`bandwidth`, `benchmark`, `commands`, `dns`, `downloads`, `logging`, `network`, `storage`, `streaming`, `windows`).

### Module Declarations
```rust
pub mod bandwidth;
pub mod benchmark;
pub mod commands;
pub mod dns;
pub mod downloads;
pub mod logging;
pub mod network;
pub mod storage;
pub mod streaming;
pub mod windows;
```
Each `pub mod <name>;` statement instructs the Rust compiler to include the corresponding subdirectory or file into the crate root, exposing it to integration tests and other internal modules.

### Function Analysis: `pub fn run()`
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 1. Start background download interceptor
            downloads::interceptor::start_download_interceptor(app.handle().clone());

            // 2. Build system tray menu
            let quit_item = MenuItem::with_id(app, "quit", "Quit NetBoost", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Open NetBoost", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .menu(&menu)
                    .tooltip("NetBoost Network Optimization Utility")
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "quit" => { app.exit(0); }
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![ ... ])
        .run(tauri::generate_context!())
        .expect("error while running NetBoost application");
}
```
* **`app.handle().clone()`**: `AppHandle` is a lightweight, thread-safe handle to the Tauri runtime that can be cloned across async boundaries. Passed into `start_download_interceptor` so background HTTP threads can emit events and focus windows.
* **`TrayIconBuilder`**: Constructs the Windows Shell notification icon using the binary's embedded icon. The `on_tray_icon_event` listener ensures that a standard single left-click unhides and focuses the window.
* **`on_window_event` Interception**: Listens for `CloseRequested`. Calling `api.prevent_close()` halts the default destroy sequence; `window.hide()` renders the window invisible, keeping download workers running in the background.
* **`manage(AppState::new())`**: Injects `AppState` into Tauri's dependency-injection container. Any command declaring `state: State<'_, AppState>` receives access automatically.
* **`tauri::generate_handler![...]`**: A macro that expands into a dispatch match statement routing JSON-RPC function names from the webview to native Rust functions.

---

## 3. `src-tauri/src/commands/network.rs`

### File Overview
* **Path**: `src-tauri/src/commands/network.rs`
* **Purpose**: Defines `AppState`, `RealtimeMetrics`, and commands for live telemetry polling and optimization mode toggles.
* **Component**: Central Command Controller.

### Data Structures

#### `pub struct AppState`
```rust
pub struct AppState {
    pub adapter_tracker: Mutex<AdapterTracker>,
    pub latency_diag: Mutex<LatencyDiagnostics>,
    pub bandwidth_mgr_arc: Arc<Mutex<BandwidthManager>>,
    pub streaming_detector: Arc<StreamingDetector>,
    pub process_monitor: Mutex<ProcessNetworkMonitor>,
    pub download_engine: Arc<DownloadEngine>,
    pub db: Arc<Database>,
    pub benchmark_runner: Arc<BenchmarkRunner>,
    pub logger: Arc<NetBoostLogger>,
}
```
* **Thread-Safety Architecture**:
  * Fields wrapped in `Mutex<T>` are stateful singletons accessed by IPC commands on worker threads.
  * Fields wrapped in `Arc<T>` or `Arc<Mutex<T>>` (`bandwidth_mgr_arc`, `download_engine`, `db`) are shared across background Tokio tasks (e.g. download chunk workers) and IPC handlers simultaneously.

#### `AppState::new()` Constructor
1. Initializes `NetBoostLogger::init()`.
2. Resolves database path: `%APPDATA%\NetBoost\netboost.db`.
3. Opens `Database::new(&db_path)` or falls back to `Database::in_memory()`.
4. Runs `db.recover_interrupted_downloads()` to reset tasks that were mid-flight during a prior crash.
5. Queries `db.get_streaming_apps()` and loads them into `StreamingDetector`.

### Key Command: `get_realtime_metrics`
```rust
#[tauri::command]
pub fn get_realtime_metrics(state: State<'_, AppState>) -> RealtimeMetrics
```
* **Step 1**: Locks `state.adapter_tracker` and calls `tracker.poll_adapters()`, querying `GetIfTable2` to calculate instantaneous upload/download speed (bps) and identify the active adapter.
* **Step 2**: Locks `state.latency_diag` and calls `diag.sample_latency()`, running a TCP connect probe to anycast resolvers (1.1.1.1:53) and updating RFC 3550 jitter.
* **Step 3**: Computes health report via `evaluate_network_health(lat_ms, jit_ms, loss_pct)`.
* **Step 4**: Computes total bytes received/sent by summing all active non-loopback adapters.
* **Step 5**: Queries `state.download_engine` for count and speed of active downloads.
* **Step 6**: Locks `state.process_monitor` and counts active media streaming applications.
* **Step 7**: Feeds all collected metrics into `state.bandwidth_mgr_arc.lock().unwrap().evaluate_congestion_with_traffic(...)`, advancing the hysteresis engine.
* **Step 8**: Returns a populated `RealtimeMetrics` struct serialized to JSON.

---

## 4. `src-tauri/src/bandwidth/manager.rs`

### File Overview
* **Path**: `src-tauri/src/bandwidth/manager.rs`
* **Purpose**: Core dynamic pacing and bufferbloat mitigation engine.
* **Component**: Intelligence Engine.

### Data Structures & Memory Layout
```rust
pub struct BandwidthManager {
    pub mode: OperatingMode,
    pub enabled: bool,
    pub baseline_latency_ms: f64,
    recent_samples: VecDeque<LatencySample>,
    window_capacity: usize,
    congestion_streak: u32,
    recovery_streak: u32,
    pub is_congested: bool,
    last_adaptation_time: Instant,
    cooldown_duration: Duration,
    current_pacing_delay_ms: u64,
    concurrency_throttle_factor: f64,
    pub last_congestion_reason: Option<String>,
}
```
* **`VecDeque<LatencySample>`**: A double-ended queue holding up to 15 recent probe measurements. Pushing to the back and popping from the front takes $O(1)$ time, forming a sliding measurement window.

### Mathematical Methods

#### `calculate_median_latency(&self) -> f64`
Extracts `rtt_ms` values from `recent_samples`, sorts them using `partial_cmp`, and returns the 50th percentile (median). Using median rather than mean filters out single-packet outliers (e.g. a momentary Wi-Fi retransmission spike) from falsely triggering bufferbloat mode.

#### `evaluate_congestion_with_traffic(...)`
Implements a **3-cycle hysteresis state machine**:
1. **Elevated Criteria**: A probe is marked elevated if:
   $$\text{RTT} > \text{Baseline} \times 1.75 \quad \text{AND} \quad (\text{RTT} - \text{Baseline}) > 25\text{ ms}$$
   or Jitter $> 18\text{ ms}$, or Packet Loss $> 5\%$.
2. **Calm Criteria**: A probe is marked calm if:
   $$\text{RTT} \le \text{Baseline} \times 1.35 \quad \text{AND} \quad \text{Jitter} \le 8\text{ ms}$$
3. **Transition to Congested**: Requires $\ge 3$ consecutive elevated cycles (`congestion_streak >= 3`).
4. **Transition to Recovery**: Requires $\ge 5$ consecutive calm cycles (`recovery_streak >= 5`).
5. **Cooldown**: `cooldown_duration` (3500 ms) prevents rapid oscillation.

#### Mode Policies
* **`OperatingMode::Streaming`**:
  * Congested: Injects **45 ms** pacing delay, scales concurrency factor to **0.40x**.
  * Recovered with active streams: Injects **30 ms** pacing delay, scales concurrency factor to **0.50x**.
* **`OperatingMode::Balanced`**:
  * Congested: Injects **15 ms to 20 ms** pacing delay, concurrency factor **0.70x**.
  * Normal: **0 ms** delay, concurrency factor **1.0x**.
* **`OperatingMode::Download`**:
  * Prioritizes throughput. Pacing delay remains **0 ms** unless packet loss exceeds 10%.

---

## 5. `src-tauri/src/downloads/engine.rs`

### File Overview
* **Path**: `src-tauri/src/downloads/engine.rs`
* **Purpose**: Multi-worker parallel download accelerator. Divides payloads into byte ranges, writes directly to sparse files, enforces dynamic pacing delays, and computes SHA-256 checksums.
* **Component**: Download Acceleration Subsystem.

### Key Workflows & Code Mechanics

#### URL Probing (`probe_url(&self, url: &str) -> Result<ProbeInfo, String>`)
* Sends an HTTP request with header `Range: bytes=0-0`.
* If the server responds with `HTTP 206 Partial Content`, `supports_range` is set to `true`.
* Parses `Content-Range: bytes 0-0/104857600` to extract the exact total file length (104,857,600 bytes).
* Extracts `ETag` and `Last-Modified` for resume validation.
* Derives filename from `Content-Disposition: attachment; filename="..."` or parses the URL path.
* Handles `HTTP 429 Too Many Requests` with automatic `Retry-After` backoff.

#### Sparse File Pre-Allocation
```rust
let part_path = format!("{}.part", dest_path.to_string_lossy());
if let Ok(file) = OpenOptions::new().create(true).write(true).open(&part_path).await {
    if probe.total_bytes > 0 {
        let _ = file.set_len(probe.total_bytes).await;
    }
}
```
* **`file.set_len(total_bytes)`**: Informs the Windows file system (NTFS) of the final file size. NTFS allocates the file as a **sparse file** instantly without zeroing bytes on disk. This allows multiple worker tasks to seek directly to byte offsets (e.g. worker 3 writing at offset 75 MB) without file collision or slow post-download segment stitching.

#### Worker Loop (`run_segment_worker(...)`)
Each assigned worker executes:
1. Calculates assigned range: `start_byte + worker_written` to `end_byte`.
2. Seeks file pointer: `file.seek(SeekFrom::Start(current_offset)).await`.
3. Sends HTTP GET request with `Range: bytes={start}-{end}`.
4. Streams response chunks in a `tokio::select!` block:
   ```rust
   // Read pacing delay check
   let pacing_delay_ms = {
       let mgr = bandwidth_mgr.lock().unwrap();
       mgr.get_pacing_delay_ms()
   };
   if pacing_delay_ms > 0 {
       tokio::time::sleep(Duration::from_millis(pacing_delay_ms)).await;
   }
   ```
   If the Bandwidth Coordinator sets a 30 ms delay, the worker yields execution, freeing network adapter buffers.
5. Bounds checks: Enforces that chunks do not write past `end_byte`.
6. Speed and ETA calculation: Recalculates speed every 250 ms.

#### Completion & SHA-256 Hash
When all workers complete, `compute_file_sha256` reads the `.part` file in 64 KB buffers, updates the `Sha256` hasher, renames the `.part` file to the final destination, and updates SQLite.

---

## 6. `src-tauri/src/downloads/interceptor.rs`

### File Overview
* **Path**: `src-tauri/src/downloads/interceptor.rs`
* **Purpose**: Local HTTP REST server listening on `127.0.0.1:52140` to capture downloads from Chrome/Edge extensions.
* **Component**: Browser Integration Bridge.

### Synchronization Pattern
```rust
static PENDING_DOWNLOADS: OnceLock<Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>> = OnceLock::new();
```
* **`OnceLock`**: Thread-safe global initialization without `lazy_static`.
* **Flow**:
  1. Browser extension posts download details to `POST /intercept`.
  2. The server creates a `tokio::sync::oneshot::channel::<bool>()`.
  3. Registers the `Sender` in `PENDING_DOWNLOADS` under the download ID.
  4. Emits `netboost-download-detected` to the React frontend.
  5. Unminimizes and focuses the window.
  6. Awaits the `Receiver` with a 25-second timeout:
     ```rust
     match tokio::time::timeout(Duration::from_secs(25), rx).await {
         Ok(Ok(val)) => val, // User clicked Accelerate (true) or Download in Browser (false)
         _ => false          // Timeout -> yield back to browser safely
     }
     ```
  7. Responds to the browser with `{"accelerate": true/false}`.
  8. If false, the browser extension leaves the native Chrome download pipeline uninterrupted.

---

## 7. `src-tauri/src/windows/iphlpapi.rs`

### File Overview
* **Path**: `src-tauri/src/windows/iphlpapi.rs`
* **Purpose**: Direct Win32 C FFI binding wrappers for kernel network telemetry and process I/O counters.
* **Component**: Windows Hardware Abstraction Layer.

### Win32 API Implementation

#### `query_interface_table() -> Vec<RawAdapterInfo>`
Calls `GetIfTable2(&mut table_ptr)`. Iterates over `MIB_IF_ROW2` structures:
* `row.InOctets`, `row.OutOctets`: 64-bit hardware byte counters.
* `row.ReceiveLinkSpeed`: Negotiated physical layer link speed in bits/sec.
* `row.PhysicalAddress`: Extracts MAC address formatted as `XX:XX:XX:XX:XX:XX`.
* `FreeMibTable`: Frees kernel-allocated memory to prevent memory leaks.

#### `query_extended_tcp_table() -> Vec<RawTcpConnection>`
Calls `GetExtendedTcpTable` with `TCP_TABLE_OWNER_PID_ALL` for both `AF_INET` (IPv4) and `AF_INET6` (IPv6). Handles Windows buffer reallocation error code `122` (`ERROR_INSUFFICIENT_BUFFER`) dynamically. Maps each active TCP socket to its owning process PID.

#### `query_process_io_counters(pid: u32) -> Option<(u64, u64)>`
Opens a handle to the process via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_QUERY_INFORMATION, 0, pid)` and invokes `GetProcessIoCounters(handle, &mut io)`. Extracts `io.ReadTransferCount` and `io.WriteTransferCount`. This measures real socket and disk I/O activity for processes like Chrome and Spotify.

---

## 8. `src-tauri/src/dns/bench.rs` & `src-tauri/src/dns/windows.rs`

### File Overview
* **Paths**: `src-tauri/src/dns/bench.rs`, `src-tauri/src/dns/windows.rs`
* **Purpose**: Benchmarks DNS resolvers via raw UDP sockets and configures Windows network adapters using `netsh`.
* **Component**: DNS Optimization Subsystem.

### Wire-Format DNS Query Builder (`build_dns_query`)
Manually constructs a standard 12-byte RFC 1035 DNS header and Question section:
* `tx_id.to_be_bytes()`: 2-byte transaction ID.
* `[0x01, 0x00]`: Flags indicating standard query with recursion desired (`RD=1`).
* `[0x00, 0x01]`: QDCOUNT = 1.
* Encodes domain into length-prefixed labels (e.g. `3www6google3com0`).
* `[0x00, 0x01]`: QTYPE = 1 (Type A).
* `[0x00, 0x01]`: QCLASS = 1 (Internet IN).

### Windows Adapter DNS Automation (`WindowsDnsManager`)
1. **Backup**: Queries `netsh interface ipv4 show dnsservers name="Wi-Fi"`, extracts existing primary and secondary IPs or DHCP status, and saves to SQLite (`dns_backups` table).
2. **Apply**: Executes `netsh interface ipv4 set dnsservers name="Wi-Fi" static 1.1.1.1 primary`.
3. **Restore**: If backup was DHCP, runs `netsh interface ipv4 set dnsservers name="Wi-Fi" source=dhcp`.
4. **Elevation Handling**: If `netsh` fails with an access denied or elevation error, it invokes PowerShell with `-Verb RunAs` to prompt the native Windows UAC dialog cleanly.

---

## 9. `src-tauri/src/storage/db.rs`

### File Overview
* **Path**: `src-tauri/src/storage/db.rs`
* **Purpose**: Embedded SQLite database operations.
* **Component**: Persistence Layer.

### Schema & Queries
* Contains tables: `downloads`, `benchmark_runs`, `user_streaming_apps`, `dns_backups`, `app_settings`.
* Uses `ON CONFLICT(...) DO UPDATE SET ...` (UPSERT) for idempotent task and backup synchronization.
* Implements `recover_interrupted_downloads()`:
  ```sql
  UPDATE downloads SET status = 'paused' WHERE status = 'downloading'
  ```

---

## 10. `src-tauri/src/streaming/detector.rs`

### File Overview
* **Path**: `src-tauri/src/streaming/detector.rs`
* **Purpose**: Classifies active Windows processes into streaming media vs. background processes.
* **Component**: Heuristic Analysis Engine.

### Heuristic Rules
1. **User Overrides**: Checks user-configured names in `user_configured_apps` (loaded from SQLite).
2. **Dedicated Players**: Exact match on `netflix.exe`, `spotify.exe`, `vlc.exe`, `stremio.exe`, `plex.exe`, etc.
3. **Web Browsers**: Evaluates Chrome, Edge, Firefox, Brave. If remote socket address ends with `:443` (HTTPS) and sustained read rate exceeds 300 Kbps (or multiple concurrent HTTPS media sockets), it classifies the browser as an active media stream with "high" confidence.

---

# PART II: FRONTEND PRESENTATION LAYER (`src/`)

---

## 11. `src/App.tsx`
* **Purpose**: Top-level React view container.
* **Key State**:
  * `currentTab`: Active tab navigation.
  * `metrics`: Current `RealtimeMetrics`.
  * `optState`: Current `OptimizationState`.
  * `speedHistory`: Array of up to 60 `{ time, downloadSpeed, uploadSpeed }` points.
* **Effects**:
  * Global link click listener capturing all `http://` and `https://` anchors, routing them through `openExternalUrl()` so they open in the default Windows browser.
  * 1000 ms polling timer invoking `getRealtimeMetrics()` and `getOptimizationState()`.

---

## 12. `src/components/Dashboard/LiveSpeedChart.tsx`
* **Purpose**: High-performance, zero-dependency SVG throughput chart.
* **Implementation Details**:
  * Rejects bloated third-party charting libraries (Chart.js, Recharts).
  * Uses pure SVG `<path>` elements with cubic Bezier lines and linear gradient fills (`#downGrad`, `#upGrad`).
  * Dynamically computes the maximum Y-scale based on the highest throughput point in the 60-second window plus 15% headroom.

---

## 13. `src/components/Downloads/QuickDownloadModal.tsx`
* **Purpose**: Modal displayed when browser extension captures a download.
* **Implementation Details**:
  * Listens to Tauri event `listen('netboost-download-detected')`.
  * Provides concurrency buttons (4x, 6x, 8x, 12x).
  * Has "Accelerate" and "Download Normally in Browser" buttons.
  * Keyboard shortcut: Pressing `Escape` invokes `handleDownloadInBrowser()`, immediately resolving the background loopback channel and returning focus to Chrome.

---

## 14. `src/components/Dns/DnsOptimizer.tsx`
* **Purpose**: UI for DNS benchmarking and adapter tuning.
* **Implementation Details**:
  * Shows benchmark cards for Cloudflare, Google, Quad9, OpenDNS, and the current system resolver.
  * Badges the lowest-latency resolver that achieves $\ge 80\%$ success rate with "RECOMMENDED".
  * Displays confirmation dialog before modifying system adapter settings.
  * Provides a 1-click "Restore Original DNS" button backed by the SQLite backup.

---

## 15. `src/components/Applications/ProcessList.tsx`
* **Purpose**: Real-time table of Windows processes using network connections.
* **Implementation Details**:
  * Displays PID, process name, path, active sockets, and I/O transfer rates.
  * Highlights detected video/audio streaming sessions with purple badges.
  * Expandable rows displaying individual remote socket IP addresses and connection states (`ESTABLISHED`, `LISTEN`, `TIME_WAIT`).
  * Drawer allowing users to add or remove custom process names to the streaming priority whitelist.

---

# PART III: BROWSER EXTENSION LAYER (`extension/`)

---

## 16. `extension/background.js`
* **Purpose**: Chromium Manifest V3 service worker intercepting downloads.
* **Implementation Details**:
  * Listens to `chrome.downloads.onCreated`.
  * Checks file extension against binary types (`.zip`, `.iso`, `.exe`, `.tar`, `.mp4`).
  * Pings `http://127.0.0.1:52140/status` to confirm NetBoost desktop app is running.
  * Extracts cookies via `chrome.cookies.getAll({ url })` so authenticated downloads (e.g. Google Drive, Dropbox) maintain session credentials.
  * Posts payload to `http://127.0.0.1:52140/intercept`.
  * If NetBoost responds `{"accelerate": true}`, cancels Chrome's native download via `chrome.downloads.cancel()`. Otherwise, leaves Chrome's download intact.

---

## 17. `extension/content.js`
* **Purpose**: Injected content script for instant user bypass.
* **Implementation Details**:
  * Listens to `window.addEventListener('click', ..., true)` in the capture phase.
  * If `e.altKey` is true, sends message `{ type: 'BYPASS_NETBOOST' }` to the service worker, setting a temporary 3-second bypass flag so users can bypass NetBoost on specific links without opening menus.

