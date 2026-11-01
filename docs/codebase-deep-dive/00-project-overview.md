# 00 — Project Overview: NetBoost

---

## 1. Executive Summary

**NetBoost** is a production-grade Windows desktop application and browser companion system engineered to address a pervasive networking problem: **bufferbloat and interactive media degradation caused by aggressive background downloads**.

In modern consumer and enterprise networking environments, standard web browsers and download utilities initiate downloads using unthrottled TCP sockets. These sockets greedily consume available link capacity and fill upstream/downstream packet queues in consumer Wi-Fi routers and network adapters. This queue saturation—technically known as **bufferbloat**—dramatically inflates round-trip time (RTT / Ping) from normal baselines (15–25 ms) to several hundred milliseconds (often 200–800 ms). 

As a direct consequence:
* **Video playback** on platforms like YouTube, Netflix, Disney+, and Twitch freezes, stutters, or drops to degraded resolutions (360p/480p).
* **Real-time communication** tools such as Discord, Zoom, and Microsoft Teams experience packet jitter, robotic voice distortion, or dropped calls.
* **Online gaming** suffers sudden lag spikes, rubberbanding, and connection timeouts.

NetBoost eliminates this queue collapse without terminating downloads. It pairs a **native Rust core engine** running inside a **Tauri 2.x desktop shell** with a **Chromium companion browser extension (Manifest V3)** and a **React 18 / Tailwind CSS** interface.

```
+-----------------------------------------------------------------------------------+
|                                NetBoost Eco-System                                |
+-----------------------------------------------------------------------------------+
|  +-------------------------------------+   +------------------------------------+ |
|  |     React 18 Desktop GUI (Vite)     |   |   Chromium Browser Extension (MV3) | |
|  |   - Real-time telemetry dashboard   |   | - Intercepts file downloads        | |
|  |   - Multi-worker download manager   |   | - Non-blocking user choice prompt  | |
|  |   - DNS latency optimizer           |   | - Local loopback REST IPC bridge   | |
|  +------------------+------------------+   +-----------------+------------------+ |
|                     | Tauri IPC Commands                     | HTTP Loopback      |
|                     | & Event Emitter                        | (127.0.0.1:52140)  |
|  +------------------v----------------------------------------v------------------+ |
|  |                          Rust Core Engine (netboost_lib)                     | |
|  |  +-----------------------+  +----------------------+  +--------------------+ | |
|  |  | Win32 IPHLPAPI Driver |  | RFC 3550 Jitter Diag |  | Streaming Detector | | |
|  |  +-----------+-----------+  +----------+-----------+  +---------+----------+ | |
|  |              |                         |                        |            | |
|  |              +-------------------------v------------------------+            | |
|  |                                        |                                     | |
|  |                        +---------------v---------------+                     | |
|  |                        | Adaptive Bandwidth Coordinator|                     | |
|  |                        |  - Sliding Window Medians     |                     | |
|  |                        |  - 3-Cycle Hysteresis         |                     | |
|  |                        |  - Dynamic Chunk Pacing       |                     | |
|  |                        +---------------+---------------+                     | |
|  |                                        |                                     | |
|  |         +------------------------------+-----------------------------+       | |
|  |         |                                                            |       | |
|  |  +------v---------------------+                            +---------v----+  | |
|  |  | Tokio Multi-Worker Engine  |                            | DNS Bench &  |  | |
|  |  | - Parallel HTTP Ranges     |                            | Auto-Tuning  |  | |
|  |  | - Sparse .part file writes |                            | (netsh + UDP)|  | |
|  |  +--------------+-------------+                            +---------+----+  | |
|  +-----------------|----------------------------------------------------|-------+ |
|                    |                                                    |         |
|  +-----------------v----------------------------------------------------v-------+ |
|  |                       Windows OS Kernel & Storage                            | |
|  |  - iphlpapi.dll (GetIfTable2, GetExtendedTcpTable, GetProcessIoCounters)     | |
|  |  - Windows Registry (HKCU Run key for startup)                              | |
|  |  - SQLite (netboost.db in %APPDATA%\NetBoost)                                | |
|  |  - Network Adapters (netsh interface ipv4)                                   | |
|  +------------------------------------------------------------------------------+ |
+-----------------------------------------------------------------------------------+
```

---

## 2. The Core Problem: Bufferbloat & Queue Collapse

To understand why NetBoost was created, one must examine what happens when an operating system downloads a file.

### 2.1 Standard Browser Downloading
When a user clicks "Download" in Chrome or Edge, the browser establishes a single TCP connection. Modern TCP stacks (such as Windows CUBIC or Compound TCP) use window-scaling algorithms designed to maximize throughput by continuously increasing the transmission window until packet loss or queue delay occurs.

Consumer network routers have memory buffers designed to smooth over bursty packet arrivals. Under sustained high-speed downloads, these buffers fill to capacity. Because the router's queue is completely saturated, time-sensitive packets—such as video audio chunks, voice packets, and DNS lookup requests—are queued behind large file chunks. 

The result is **bufferbloat**:
$$\text{Latency}_{\text{Loaded}} = \text{Latency}_{\text{Idle}} + \text{Queue Delay}$$

While idle latency might be $15\text{ ms}$, loaded latency frequently reaches $250\text{ ms}$ to $800\text{ ms}$.

### 2.2 How NetBoost Solves It
NetBoost does not simply cap overall download speed with an arbitrary global limit. Instead, it implements **intelligent, adaptive pacing**:
1. **Live Kernel Telemetry**: It continuously polls the hardware adapter counters directly from Windows via `GetIfTable2` and measures active RTT and RFC 3550 interarrival jitter.
2. **Process Network Inspection**: It scans active Windows TCP sockets using `GetExtendedTcpTable` and identifies when video streaming (YouTube, Netflix, Twitch) or voice applications (Discord, Teams, Zoom) are actively receiving network data.
3. **Adaptive Read Pacing**: When streaming activity or bufferbloat is detected, NetBoost injects microsecond-to-millisecond sleep intervals (typically 20 ms to 45 ms) between chunk reads in its download worker loops. This grants the Wi-Fi router's packet buffer sufficient time to drain, immediately restoring low latency to streaming media while keeping background downloads running.

---

## 3. High-Level Feature Inventory

NetBoost provides seven integrated capabilities:

| Feature | Technical Implementation | Purpose |
| :--- | :--- | :--- |
| **Real-Time Telemetry Dashboard** | `iphlpapi.dll` `GetIfTable2`, 64-bit octet polling, SVG live chart | Provides 60-second rolling download/upload rates, RTT, jitter, and packet loss index. |
| **Streaming Priority Engine** | Socket analysis (`GetExtendedTcpTable`) + I/O transfer rates (`GetProcessIoCounters`) | Detects media streams automatically and coordinates download concurrency. |
| **Dynamic Bandwidth Coordinator** | Sliding-window median filters + 3-cycle hysteresis + adaptive chunk pacing | Mitigates bufferbloat without hard-throttling or cancelling user downloads. |
| **Multi-Worker Download Accelerator** | Tokio async tasks + HTTP `Range` headers + Sparse pre-allocated `.part` files | Accelerates downloads up to 4x–12x over single-stream browser downloads. |
| **Browser Extension Interceptor** | Manifest V3 Chromium extension + local loopback HTTP server (`127.0.0.1:52140`) | Captures browser downloads seamlessly with an instant choice modal. |
| **DNS Optimizer & Auto-Tuning** | Native UDP port 53 query builder + `netsh` automation + backup/restore | Benchmarks public resolvers (Cloudflare, Google, Quad9, OpenDNS) and tunes Windows adapters. |
| **Benchmark Suite** | Automated test runner simulating idle, loaded, and optimized scenarios | Stores comparative latency and throughput history in local SQLite database. |

---

## 4. Technology Stack Summary

NetBoost uses a hybrid architecture that pairs a high-performance, memory-safe systems language (Rust) with a modern web UI framework (React 18):

```
+---------------------------------------------------------------------+
| Layer              | Technology Used                                |
+--------------------+------------------------------------------------+
| Systems Backend    | Rust (Edition 2021), Tokio Async Runtime       |
| Desktop Framework  | Tauri 2.12.5 (Windows WebView2)               |
| Windows Native API | windows-sys 0.59 (Win32 IPHLPAPI, WinSock, PE) |
| Local Persistence  | SQLite via rusqlite 0.32 (bundled)             |
| HTTP Client & TLS  | Reqwest 0.12 with Rustls-TLS                   |
| Frontend UI        | React 18.3.1, TypeScript 5.7.2, Vite 5.4.11    |
| Styling            | Tailwind CSS 3.4.16, Lucide React Icons        |
| Browser Extension  | Chrome Manifest V3, Vanilla JavaScript ES2022  |
| Windows Installer  | NSIS (Nullsoft Scriptable Install System)      |
+---------------------------------------------------------------------+
```

---

## 5. End-to-End User Experience

To understand how all these moving parts function in practice, consider a standard user workflow:

1. **Launch**: The user launches NetBoost from the Windows Start Menu or system tray. The application reads user preferences from `%APPDATA%\NetBoost\netboost.db` and starts background telemetry polling.
2. **Monitoring**: The user observes the real-time speed chart and health indicator. NetBoost displays the active physical adapter (e.g., Intel Wi-Fi 6E AX211) and its negotiated link rate.
3. **Browser Download**: The user browses to a software download site (e.g., downloading an ISO or ZIP archive). 
4. **Interception**: The companion browser extension detects a binary download item. It contacts the local NetBoost engine at `http://127.0.0.1:52140/intercept`.
5. **Choice**: NetBoost brings its window forward with a quick pop-up modal asking whether to accelerate with NetBoost (4x, 6x, 8x channels) or continue normally in the browser.
6. **Parallel Acceleration**: If the user confirms, the browser download is cancelled and NetBoost probes the URL for HTTP Range support. It pre-allocates the file on disk and launches concurrent Tokio workers that write chunk offsets directly into the `.part` file.
7. **Streaming Protection**: During the download, the user opens YouTube or enters a Discord call. NetBoost's process monitor detects HTTPS streaming sockets on `chrome.exe` and `discord.exe`. The Bandwidth Coordinator automatically injects a 30 ms pacing delay between download chunks, keeping ping stable at 20 ms and preventing YouTube from buffering.
8. **Completion**: Upon finishing, NetBoost computes a SHA-256 hash of the payload, verifies integrity, renames the `.part` file to the final destination, and stores the task in SQLite history.

