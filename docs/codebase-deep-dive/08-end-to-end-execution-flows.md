# 08 — End-to-End Execution Flows: Complete Sequence Traces

---

## 1. Flow A: Browser Download Interception & Multi-Worker Acceleration

This trace details the journey of a user clicking a file download link on a website until the accelerated file is verified on disk and opened in Windows Explorer.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Browser as Chrome / Edge Browser
    participant Ext as extension/background.js
    participant Server as Loopback Server (127.0.0.1:52140)
    participant TauriCore as Tauri Core / lib.rs
    participant Modal as QuickDownloadModal.tsx
    participant Engine as downloads::DownloadEngine
    participant Workers as Tokio Range Workers (1..N)
    participant Remote as Remote HTTP Server
    participant Disk as Windows NTFS Disk

    User->>Browser: Clicks download link (e.g. ubuntu-24.04.iso)
    Browser->>Ext: chrome.downloads.onCreated(downloadItem)
    Ext->>Ext: Check extension (.iso) & MIME type
    Ext->>Server: GET /status (Check if NetBoost is running)
    Server-->>Ext: 200 OK {"status":"online"}
    Ext->>Browser: chrome.cookies.getAll({ url })
    Ext->>Server: POST /intercept (id, url, filename, cookies)

    Server->>Server: Create oneshot::channel::<bool>()
    Server->>TauriCore: emit("netboost-download-detected", payload)
    Server->>TauriCore: Unminimize and focus NetBoost window
    TauriCore->>Modal: Event received -> Render QuickDownloadModal

    alt User clicks "Accelerate with NetBoost"
        User->>Modal: Clicks "Accelerate (8x Channels)"
        Modal->>TauriCore: invoke('resolve_intercepted_download', { id, accept: true })
        TauriCore->>Server: resolve_download_choice(id, true)
        Server->>Ext: Response 200 OK {"accelerate": true}
        Ext->>Browser: chrome.downloads.cancel(id) & erase(id)

        Modal->>TauriCore: invoke('start_download', { url, concurrency: 8 })
        TauriCore->>Engine: start_download(url, filename, 8)
        Engine->>Remote: GET url with header Range: bytes=0-0
        Remote-->>Engine: 206 Partial Content (Content-Range: 0-0/4294967296)

        Engine->>Disk: Open destination.part and invoke set_len(4GB)
        Disk-->>Engine: Sparse file pre-allocated instantly

        Engine->>Engine: Divide 4GB into 8 equal byte ranges
        Engine->>Workers: Spawn 8 Tokio async tasks (run_segment_worker)

        par Parallel Download Chunks
            Workers->>Remote: Worker 1: GET Range: bytes=0-536870911
            Workers->>Remote: Worker 2: GET Range: bytes=536870912-1073741823
            Workers->>Remote: Worker 8: GET Range: bytes=3758096384-4294967295
            Remote-->>Workers: Stream response chunks
            Workers->>Disk: Seek offset -> file.write_all(chunk)
        end

        Workers-->>Engine: All 8 worker tasks completed
        Engine->>Disk: compute_file_sha256(destination.part)
        Disk-->>Engine: Returns SHA-256 string
        Engine->>Disk: Rename destination.part to destination.iso
        Engine->>TauriCore: Task status = Completed
        User->>TauriCore: Clicks "Open File" -> explorer.exe /select,"destination.iso"

    else User clicks "Download Normally in Browser"
        User->>Modal: Clicks "Download Normally in Browser" (or presses Esc)
        Modal->>TauriCore: invoke('resolve_intercepted_download', { id, accept: false })
        TauriCore->>Server: resolve_download_choice(id, false)
        Server->>Ext: Response 200 OK {"accelerate": false}
        Ext->>Browser: Leave Chrome native download intact
        TauriCore->>TauriCore: Hide NetBoost window to system tray
    end
```

---

## 2. Flow B: Dynamic Streaming Prioritization & Pacing under Load

This trace explains how NetBoost detects when a user begins watching YouTube or joins a Discord voice call during an active download, and dynamically paces chunk reads to eliminate bufferbloat without stopping the download.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Worker as Download Worker Task
    participant Coord as BandwidthManager
    participant Monitor as ProcessNetworkMonitor
    participant Kernel as Windows Kernel (iphlpapi.dll)
    participant Diag as LatencyDiagnostics
    participant Router as Local Wi-Fi Router
    participant StreamApp as Chrome / YouTube (or Discord)

    Note over Worker: Background download running at 80 Mbps.<br/>Wi-Fi router queue fills; Ping rises from 18ms to 180ms.

    User->>StreamApp: Starts YouTube 4K video or enters Discord call
    StreamApp->>Kernel: Opens multiple TLS sockets on port 443

    loop Every 1000ms Telemetry Poll
        Monitor->>Kernel: GetExtendedTcpTable() & GetProcessIoCounters()
        Kernel-->>Monitor: Process socket tables & I/O byte rates
        Monitor->>Monitor: StreamingDetector: Chrome has 3 HTTPS sockets + read rate > 1.5 Mbps
        Monitor-->>Coord: active_streaming_sessions = 1

        Diag->>Router: TCP connect probe to 1.1.1.1:53
        Router-->>Diag: Ping spiked to 175 ms, Jitter = 19.4 ms
        Diag-->>Coord: (latency: 175ms, jitter: 19.4ms)

        Coord->>Coord: elevated probe detected (congestion_streak += 1)
        Note over Coord: Streak reaches 3 consecutive cycles.<br/>BandwidthManager transitions to CONGESTED!
        Coord->>Coord: Apply Streaming Policy:<br/>pacing_delay_ms = 45ms<br/>concurrency_factor = 0.40
    end

    loop Active Chunk Read Loop in Download Worker
        Worker->>Coord: get_pacing_delay_ms()
        Coord-->>Worker: Returns 45 ms
        Worker->>Worker: tokio::time::sleep(Duration::from_millis(45)).await
        Note over Worker,Router: Worker pauses reading from TCP socket.<br/>Router's packet buffer drains completely!
    end

    loop Next Telemetry Polls (Recovery Phase)
        Diag->>Router: TCP connect probe to 1.1.1.1:53
        Router-->>Diag: Ping drops to 22 ms, Jitter = 2.1 ms
        Coord->>Coord: recovery_streak reaches 5 cycles -> Set pacing_delay_ms = 30ms (Sustained Protection)
    end

    Note over StreamApp: YouTube 4K stream continues playback smoothly with zero buffering!
```

---

## 3. Flow C: DNS Optimization, Benchmarking & System Restoration

This trace details benchmarking public resolvers, backing up existing Windows network configuration, applying low-latency DNS servers, and restoring original settings.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant UI as DnsOptimizer.tsx
    participant Tauri as Tauri IPC (commands/dns.rs)
    participant Bench as DnsBenchmarkEngine
    participant WinDns as WindowsDnsManager
    participant SQLite as Database (netboost.db)
    participant Netsh as Windows netsh.exe
    participant DNS as Public Resolvers (Cloudflare, Google, Quad9)

    User->>UI: Navigates to "DNS Tuning" tab
    UI->>Tauri: invoke('benchmark_dns')
    Tauri->>WinDns: get_interface_dns_config("Wi-Fi")
    WinDns->>Netsh: netsh interface ipv4 show dnsservers name="Wi-Fi"
    Netsh-->>WinDns: Output: "DNS servers configured through DHCP" (is_dhcp: true)

    Tauri->>Bench: benchmark_all(current_dns: None, iface: "Wi-Fi")
    par Raw UDP Port 53 Probes
        Bench->>DNS: Probe Cloudflare (1.1.1.1:53) with 5 test domains
        Bench->>DNS: Probe Google (8.8.8.8:53) with 5 test domains
        Bench->>DNS: Probe Quad9 (9.9.9.9:53) with 5 test domains
        Bench->>DNS: Probe OpenDNS (208.67.222.222:53) with 5 test domains
        DNS-->>Bench: Return RFC 1035 wire-format responses
    end

    Bench->>Bench: Calculate median latency, min, max, success rate
    Note over Bench: Cloudflare achieves lowest median latency (11.2 ms).<br/>Mark is_recommended = true.
    Bench-->>Tauri: DnsBenchmarkReport
    Tauri-->>UI: Render report with "Cloudflare DNS (11.2 ms)" badge

    User->>UI: Clicks "Apply Cloudflare DNS"
    UI->>UI: Displays confirmation dialog with backup disclosure
    User->>UI: Confirms Apply
    UI->>Tauri: invoke('apply_dns_provider', { interface: "Wi-Fi", primary: "1.1.1.1", secondary: "1.0.0.1" })

    Tauri->>WinDns: apply_dns(db, "Wi-Fi", "1.1.1.1", "1.0.0.1")
    WinDns->>SQLite: save_dns_backup(DnsBackupRecord { iface: "Wi-Fi", is_dhcp: true, ... })
    WinDns->>Netsh: netsh interface ipv4 set dnsservers name="Wi-Fi" static 1.1.1.1 primary
    WinDns->>Netsh: netsh interface ipv4 add dnsservers name="Wi-Fi" 1.0.0.1 index=2
    Netsh-->>WinDns: Success (status 0)
    WinDns-->>UI: Successfully applied notification

    opt Later: User clicks "Restore Original DNS"
        User->>UI: Clicks "Restore Original DNS"
        UI->>Tauri: invoke('restore_original_dns', { interface: "Wi-Fi" })
        Tauri->>WinDns: restore_original_dns(db, "Wi-Fi")
        WinDns->>SQLite: get_dns_backup("Wi-Fi") -> Returns is_dhcp: true
        WinDns->>Netsh: netsh interface ipv4 set dnsservers name="Wi-Fi" source=dhcp
        Netsh-->>WinDns: Restored
        WinDns->>SQLite: delete_dns_backup("Wi-Fi")
        WinDns-->>UI: "Restored original DNS configuration for Wi-Fi."
    end
```

---

## 4. Flow D: Application Crash Recovery & Task Resumption on Boot

This trace details how NetBoost recovers gracefully when the operating system loses power or terminates forcefully during active multi-worker downloading.

```mermaid
sequenceDiagram
    autonumber
    participant OS as Windows OS Boot Manager
    participant Main as src-tauri/src/main.rs
    participant AppState as AppState::new()
    participant SQLite as Database (netboost.db)
    participant Disk as Local Storage (.part file)
    participant UI as DownloadManager.tsx
    participant Engine as DownloadEngine
    participant Remote as Remote HTTP Server

    Note over OS: System power loss occurred while downloading file-100MB.zip.<br/>SQLite database contains task with status='downloading', downloaded_bytes=42MB.

    OS->>Main: User logs into Windows -> NetBoost.exe launches
    Main->>AppState: AppState::new()
    AppState->>SQLite: Database::new("%APPDATA%\\NetBoost\\netboost.db")
    SQLite->>SQLite: Execute schema migrations (CREATE TABLE IF NOT EXISTS...)

    AppState->>SQLite: recover_interrupted_downloads()
    SQLite->>SQLite: UPDATE downloads SET status = 'paused' WHERE status = 'downloading'
    SQLite-->>AppState: Affected count = 1

    AppState->>Engine: DownloadEngine::new(db)
    Engine->>SQLite: get_all_downloads()
    SQLite-->>Engine: Returns 1 task with status = Paused (42MB / 100MB)

    AppState->>UI: Webview mounts -> DownloadManager fetches tasks
    UI->>UI: Displays "file-100MB.zip" with status PAUSED and Resume (▶) button

    User->>UI: Clicks Resume (▶) button
    UI->>Engine: resume_download("dl-12345")
    Engine->>Remote: Probe URL (GET Range: bytes=0-0)
    Remote-->>Engine: Returns ETag: "abc", Last-Modified: "Wed, 21 Oct 2025"

    Engine->>Engine: Compare ETag with saved record
    Note over Engine: ETag matches! Remote file has not changed on server.<br/>No need to discard downloaded bytes.

    Engine->>Disk: Verify file-100MB.zip.part exists
    Engine->>Engine: Divide remaining 58 MB across workers
    Engine->>Engine: Spawn workers seeking from offset 42 MB
    Engine->>Remote: GET Range: bytes=44040192-104857600
    Remote-->>Engine: Stream remaining bytes into .part file
    Engine->>Disk: compute_file_sha256() -> Rename to file-100MB.zip
    Engine->>UI: Status = Completed!
```

