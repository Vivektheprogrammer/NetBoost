# 13 — Testing & Debugging Guide: Test Suites, Harnesses & Diagnostics

---

## 1. Test Architecture Overview

NetBoost maintains an automated integration test suite located in `src-tauri/tests/`. The test suite exercises every critical systems component in an isolated environment without requiring an active internet connection.

```
src-tauri/tests/
├── test_download_engine.rs          # HTTP Range worker tests, pause/resume, SHA-256
├── test_bandwidth_coordinator.rs    # AIMD state transitions, 3-cycle hysteresis, pacing delays
├── test_streaming_heuristics.rs     # Process classification, browser HTTPS socket heuristics
├── test_network.rs                  # Network adapter tracking, speed delta calculations
├── test_dns_optimizer.rs            # RFC 1035 wire-format query generation & parsing
├── test_app_monitor.rs              # Win32 process enumeration & TCP table parsing
├── test_production_persistence.rs   # SQLite schema creation, CRUD & crash recovery
└── test_benchmark_engine.rs         # Latency probe sampling, baseline comparison math
```

---

## 2. Test Suite Breakdown

### 2.1 `test_download_engine.rs`
* **Focus**: Multi-worker HTTP Range acceleration, sparse file writing, and cryptographic verification.
* **Mocking Strategy**: Spins up a mock asynchronous HTTP server using `tokio::net::TcpListener` bound to an ephemeral port (`127.0.0.1:0`). The server inspects incoming `Range` headers:
  * Responds with `HTTP 206 Partial Content` when `Range: bytes=X-Y` is requested.
  * Responds with `HTTP 200 OK` for full payload downloads.
  * Injects simulated socket drops and `HTTP 429` rate limits to test self-healing auto-reconnects.
* **Assertions Verified**:
  * Downloaded file byte size matches expected payload exactly.
  * Computed SHA-256 hash matches the deterministic source payload hash.
  * Pausing stops worker writes; resuming continues from the exact byte offset without restarting.

### 2.2 `test_bandwidth_coordinator.rs`
* **Focus**: Algorithmic hysteresis, sliding-window median filters, and pacing delay adaptation.
* **Key Tests**:
  * `test_hysteresis_state_transitions`: Verifies that 1 or 2 elevated latency probes do NOT trigger congestion mode, but 3 consecutive elevated probes trigger congestion mode immediately.
  * `test_recovery_streak`: Verifies that 5 consecutive calm probes are required to transition back to normal mode.
  * `test_pacing_delay_output`: Validates that Streaming mode injects 45 ms during congestion and 30 ms during active streaming playback.

### 2.3 `test_streaming_heuristics.rs`
* **Focus**: Process classification rules.
* **Key Tests**:
  * Dedicated streaming apps (`netflix.exe`, `spotify.exe`, `vlc.exe`) with active sockets are classified with `"high"` confidence.
  * Web browsers (`chrome.exe`, `msedge.exe`) with multiple HTTPS connections on port `:443` and sustained read rates $> 300\text{ Kbps}$ are classified as active media streams.
  * User-configured application additions in SQLite override default heuristics.

### 2.4 `test_dns_optimizer.rs`
* **Focus**: Binary wire-format RFC 1035 DNS packet generation and response unpacking.
* **Key Tests**:
  * `build_dns_query` produces valid 12-byte headers with correct Transaction IDs, QDCOUNT = 1, QTYPE = 1 (Type A), and QCLASS = 1.
  * Domain label encoding correctly formats `"google.com"` into `\x06google\x03com\x00`.

### 2.5 `test_production_persistence.rs`
* **Focus**: SQLite embedded database transactions and crash recovery.
* **Key Tests**:
  * Verifies DDL execution for all 5 tables and indexes.
  * Tests UPSERT (`ON CONFLICT`) logic when updating download progress.
  * Tests `recover_interrupted_downloads()`, verifying that tasks stuck in `downloading` status are reset to `paused`.

---

## 3. How to Run Tests

### Running All Integration Tests
From the `src-tauri/` directory:
```powershell
cd C:\NetBoost\src-tauri
cargo test
```

### Running a Specific Test File
```powershell
cargo test --test test_download_engine
```

### Running Tests Sequentially (To Avoid Paging File Exhaustion)
On systems with limited RAM, compiling multiple heavy test harnesses in parallel with the MSVC linker can encounter `os error 1455` (commit charge exceeded). Run tests sequentially with:
```powershell
cargo test -p netboost -- --test-threads=1
```

---

## 4. Debugging & Diagnostic Procedures

### 4.1 Locating Diagnostic Logs
NetBoost writes logs to three primary locations:

1. **Application Operational Logs**:
   `%APPDATA%\NetBoost\logs\netboost.log`
   * Contains startup logs, bandwidth state transitions, and user settings changes.
   * Can also be viewed directly inside the desktop app via **Settings -> View Application Logs**.
2. **Panic Log**:
   `C:\NetBoost\panic.log` and `%APPDATA%\NetBoost\logs\crash.log`
   * Created automatically by the global panic hook in `main.rs` if an unexpected Rust panic occurs.
3. **SQLite Database**:
   `%APPDATA%\NetBoost\netboost.db`
   * Can be inspected using any standard SQLite viewer (e.g. SQLite Browser) to audit tasks, benchmark runs, and DNS backups.

---

### 4.2 Frontend Webview DevTools
To inspect the React frontend, DOM tree, network requests, and console logs:
1. When running in development mode (`npm run tauri dev`), right-click anywhere in the NetBoost window and select **Inspect Element** (F12).
2. The Microsoft Edge Chromium DevTools window will open, providing full access to the React component tree and console.

---

### 4.3 Investigating Common Issues

#### Issue 1: Port 52140 Already in Use
* **Symptom**: Extension cannot connect, or desktop engine reports `failed to bind 127.0.0.1:52140`.
* **Cause**: Another instance of NetBoost is already running in the background.
* **Diagnosis & Fix**:
  ```powershell
  # Check which process holds port 52140
  Get-NetTCPConnection -LocalPort 52140
  # Stop duplicate instances
  Stop-Process -Name "*netboost*" -Force
  ```

#### Issue 2: DNS Modification Denied
* **Symptom**: "Modifying network adapter DNS settings requires Windows Administrator permission."
* **Cause**: User clicked "No" on the Windows UAC elevation prompt, or Windows policy forbids unprivileged PowerShell `Start-Process -Verb RunAs`.
* **Fix**: Allow the Windows UAC prompt when applying DNS, or launch NetBoost as Administrator.

#### Issue 3: Remote Server Disallows Multi-Worker Ranges
* **Symptom**: Download starts with only 1 worker channel despite requesting 8x channels.
* **Cause**: The remote HTTP server does not respond with `HTTP 206 Partial Content` or lacks `Accept-Ranges: bytes`.
* **Explanation**: This is normal, intended behavior. NetBoost detects that the remote server cannot support byte ranges and falls back to a safe single-worker stream to prevent corrupting the file.

