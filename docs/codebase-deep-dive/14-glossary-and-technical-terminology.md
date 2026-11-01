# 14 — Glossary & Technical Terminology: Networking, Systems, and Architecture Reference

---

## 1. Networking & Diagnostics Terminology

### AIMD (Additive Increase / Multiplicative Decrease)
A feedback control algorithm used in TCP congestion control and adaptive rate pacing. In NetBoost, when network queues are calm, pacing delay decreases or concurrency increases linearly (additive increase); when congestion or bufferbloat is detected, concurrency drops sharply and pacing delay increases multiplicatively to instantly relieve router queues.

### Anycast DNS
A network addressing and routing technique where a single IP address (e.g., `1.1.1.1` or `8.8.8.8`) is advertised from multiple geographic data centers worldwide via BGP (Border Gateway Protocol). Routers automatically send packets to the nearest available data center, providing minimal latency and high resilience.

### Bufferbloat
Excessive buffering of network packets inside consumer routers, Wi-Fi access points, and cable/fiber modems. When large file downloads saturate link capacity, packet queues fill to maximum capacity, causing high latency (ping spikes from 20 ms to 400+ ms) and packet jitter that severely degrades real-time video streaming, voice calls, and gaming.

### DASH / HLS (Dynamic Adaptive Streaming over HTTP / HTTP Live Streaming)
Modern video streaming protocols used by YouTube, Netflix, and Twitch. Media is partitioned into short segments (typically 2 to 6 seconds long) fetched over standard HTTPS connections. The video player automatically adjusts resolution (from 4K down to 360p) based on estimated network latency and packet cadence.

### HTTP Range Requests (RFC 7233)
An HTTP/1.1 standard allowing clients to request specific byte sub-ranges of a remote file rather than the entire payload (e.g., `Range: bytes=52428800-104857599`). This enables multi-worker parallel chunk downloading and clean pause/resume resumption.

### Jitter (RFC 3550 Interarrival Jitter)
The statistical variance in packet arrival intervals over time. NetBoost implements the smoothed variance formula from **RFC 3550 Section 6.4.1**, measuring how consistently consecutive ping probes return. High jitter indicates fluctuating network queues that risk video buffer starvation.

### RTT (Round-Trip Time / Ping)
The elapsed duration (measured in milliseconds) required for a network packet to travel from the local machine to a remote endpoint and for the acknowledgment response to return.

### Sparse File
A file system allocation technique supported by NTFS and ReFS where empty blocks do not consume physical disk storage. NetBoost uses `file.set_len(total_bytes)` to pre-allocate destination files instantly, allowing multiple workers to write chunk offsets concurrently without file-stitching overhead.

### TTFB (Time To First Byte)
The duration from when an HTTP client sends a request until the first byte of the response is received. DNS lookup latency directly inflates TTFB.

---

## 2. Windows Kernel & Operating System Terminology

### FFI (Foreign Function Interface)
A programming mechanism that allows code written in one language (Rust) to call functions and access data structures compiled in another language (C / Windows Win32 API).

### `GetExtendedTcpTable`
A native Windows function exported by `iphlpapi.dll` that enumerates all active TCP listening sockets, inbound connections, and outbound sockets, mapping each socket to its owning process PID.

### `GetIfTable2`
A modern Windows Win32 function that retrieves the MIB-II network interface table. Unlike legacy 32-bit `GetIfTable`, `GetIfTable2` provides true 64-bit hardware interface octet counters (`InOctets`, `OutOctets`) that do not roll over during sustained high-speed gigabit transfers.

### `GetProcessIoCounters`
A Windows Win32 function exported by `kernel32.dll` that measures total disk and network socket bytes read (`ReadTransferCount`) and written (`WriteTransferCount`) by a given process handle.

### `iphlpapi.dll` (IP Helper API)
The core Windows operating system dynamic link library that provides network configuration and telemetry services for local network interfaces, routing tables, and transport layer connections.

### `netsh.exe` (Network Shell)
A native Windows command-line utility used to inspect and configure network adapters, IPv4/IPv6 addresses, and interface DNS servers.

### PE (Portable Executable) Subsystem
The executable file format used by Windows (`.exe`, `.dll`). Setting the subsystem to `IMAGE_SUBSYSTEM_WINDOWS_GUI` (`#![windows_subsystem = "windows"]`) informs Windows that the application manages its own GUI, preventing the operating system from spawning a black Command Prompt window.

### NSIS (Nullsoft Scriptable Install System)
An open-source script-driven installer tool used by Tauri to compile Windows setup wizards (`Setup.exe`) that handle file extraction, Start Menu shortcuts, and uninstallation.

---

## 3. Rust & Systems Architecture Terminology

### `Arc<T>` (Atomic Reference Counting)
A thread-safe reference-counting pointer in Rust that allows multiple threads to share read-only ownership of the same heap allocation.

### `AtomicBool`
A primitive boolean value that can be safely read and updated across threads concurrently using atomic CPU hardware instructions without taking a mutex lock.

### Borrowing & Lifetimes
The compile-time rules in Rust governing how references to data (`&T` and `&mut T`) can be passed between functions. The Rust compiler guarantees that data can never be freed while a reference is still active, eliminating use-after-free bugs.

### Hysteresis
The dependence of the state of a system on its history. NetBoost uses a 3-cycle streak to enter congestion mode and a 5-cycle streak to exit, preventing rapid oscillation between throttling and full-speed states.

### `Mutex<T>` (Mutual Exclusion)
A concurrency primitive in Rust that protects shared data by ensuring only one thread can access the data at any given time. In Rust, the `Mutex` wraps the data itself, preventing access without holding the lock.

### `OnceLock<T>`
A thread-safe cell that can be written to exactly once. Used in NetBoost for global singleton channels without requiring unsafe mutable statics.

### Tokio
An asynchronous event-driven runtime for Rust featuring a multi-threaded work-stealing scheduler, non-blocking network sockets, timers, and channels.

---

## 4. Frontend & Desktop Framework Terminology

### Chromium WebView2
A native Windows desktop UI control powered by Microsoft Edge (Chromium) that allows desktop applications to render modern web interfaces using operating-system-provided browser components.

### HMR (Hot Module Replacement)
A development feature provided by Vite that injects updated JavaScript and CSS modules into the running browser window without requiring a full page refresh.

### IPC (Inter-Process Communication)
The mechanism allowing the frontend webview (JavaScript) to exchange JSON-RPC messages and asynchronous events with the backend systems engine (Rust).

### Manifest V3 (MV3)
The modern extension platform architecture enforced by Google Chrome and Microsoft Edge, requiring lightweight, event-driven background service workers instead of persistent background pages.

