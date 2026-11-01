# 03 — Frameworks, Libraries, and Dependencies

---

## 1. Overview of the Dependency Ecosystem

NetBoost combines a lean set of high-performance Rust crates with a minimal, modern frontend npm stack. Every dependency was deliberately chosen to maximize performance, eliminate runtime overhead, and provide direct Windows operating system access.

```
+----------------------------------------------------------------------------------+
|                              NetBoost Dependency Graph                           |
+----------------------------------------------------------------------------------+
| FRONTEND (npm):                                                                  |
|   React 18.3.1 -------------> UI Component Render Tree                           |
|   Vite 5.4.11 --------------> ESBuild & Rollup Bundling Pipeline                  |
|   Tailwind CSS 3.4.16 ------> Utility-First CSS Engine                           |
|   Lucide React 1.16.0 ------> High-DPI Vector Icons                              |
|   @tauri-apps/api 2.2.0 ----> JSON-RPC IPC Channel to Backend                    |
+---------------------------------------+------------------------------------------+
                                        | IPC Serialization Bridge
+---------------------------------------v------------------------------------------+
| BACKEND (Cargo):                                                                 |
|   Tauri 2.12.5 -------------> Native Desktop Shell, Tray & IPC Router            |
|   Tokio 1.x ----------------> Async Event Loop, Timers & Multi-Worker Green Threads|
|   Reqwest 0.12 (rustls) ----> HTTP Range Streaming Client                        |
|   rusqlite 0.32 (bundled) --> Embedded SQLite Engine for Persistence             |
|   windows-sys 0.59 ---------> Win32 Kernel FFI (iphlpapi.dll, ws2_32.dll)        |
|   serde / serde_json 1.0 ---> High-Speed Zero-Copy Serialization                 |
|   sha2 0.10 ----------------> Cryptographic Hash Verification                    |
+----------------------------------------------------------------------------------+
```

---

## 2. Rust Backend Dependencies (`Cargo.toml`)

### 2.1 `tauri` (v2.12.5) & `tauri-build` (v2.7.4)
* **What it is**: A modern cross-platform desktop framework that pairs a native Rust backend with a webview frontend.
* **Problem it solves**: Traditional desktop frameworks (such as Electron) bundle an entire Chromium browser binary and Node.js runtime into every application, resulting in 150 MB+ installer sizes and 300 MB+ idle RAM consumption.
* **Why NetBoost needs it**: NetBoost requires native desktop integration (Windows system tray, prevent-close-to-tray, custom window borders, local file execution) while leveraging modern web technologies for complex real-time charts and data tables.
* **How it works internally**: On Windows, Tauri embeds the operating system's built-in **Microsoft Edge WebView2** control via COM interfaces. Communication between the webview and the Rust engine occurs over an asynchronous message-passing IPC bridge using Windows message queues and webview script injection.
* **Where it is used**:
  * `src-tauri/src/lib.rs`: `tauri::Builder`, `TrayIconBuilder`, window close handlers, command registration.
  * `src-tauri/src/commands/*.rs`: `State<'_, AppState>`, `#[tauri::command]`.
* **What would happen if removed**: NetBoost would lose its desktop window, system tray, and frontend-to-backend communication layer.
* **Alternatives**: Electron (too bloated), Qt/C++ (poor UI development velocity), Wails (Go-based, but GC pauses would hurt pacing precision).

---

### 2.2 `tokio` (v1, features: `full`)
* **What it is**: An event-driven, non-blocking asynchronous runtime for Rust.
* **Problem it solves**: Operating system threads are expensive (consuming ~1 MB of stack memory and requiring OS context switches). Spawning 12+ OS threads per download would waste CPU cycles and limit scalability.
* **Why NetBoost needs it**: NetBoost runs multiple concurrent background tasks simultaneously:
  1. The local HTTP loopback server listening on port 52140.
  2. Up to 12 parallel HTTP Range workers per download.
  3. Pacing delay timers (`tokio::time::sleep`).
  4. Non-blocking file I/O operations (`tokio::fs::File`).
* **How it works internally**: Tokio uses a multi-threaded **work-stealing scheduler**. When a download worker calls `tokio::time::sleep` or awaits a socket read, Tokio puts the task to sleep and switches the CPU core to execute another worker task, achieving thousands of concurrent operations on just a few worker threads.
* **Where it is used**:
  * `src-tauri/src/downloads/engine.rs`: Spawning workers, async file reads/writes, channels.
  * `src-tauri/src/downloads/interceptor.rs`: `tokio::net::TcpListener`, `tokio::sync::oneshot`.
  * `src-tauri/src/benchmark/engine.rs`: Async probe scheduling and HTTP streaming.
* **What would happen if removed**: Asynchronous programming in Rust would be impossible; all download threads would have to use blocking OS threads, consuming significant RAM and destroying pacing accuracy.
* **Alternatives**: `async-std`, `smol`.

---

### 2.3 `windows-sys` (v0.59)
* **What it is**: Microsoft's official, raw Win32 FFI bindings for Rust.
* **Problem it solves**: Rust cannot directly call Windows kernel C APIs without foreign function interface (FFI) declarations specifying struct memory layouts, calling conventions (`stdcall`), and function signatures.
* **Why NetBoost needs it**: NetBoost's core telemetry and process inspection features depend on Windows kernel APIs:
  * `GetIfTable2`: Reads 64-bit hardware interface byte counters (`InOctets`, `OutOctets`) directly from `iphlpapi.dll`.
  * `GetExtendedTcpTable`: Queries all active TCP sockets mapped to owner Process IDs (PIDs).
  * `GetProcessIoCounters`: Measures actual disk and socket read/write rates for running processes.
  * `OpenProcess`, `QueryFullProcessImageNameW`: Resolves process executable names and paths.
* **How it works internally**: Provides zero-cost compile-time bindings directly to `kernel32.dll`, `iphlpapi.dll`, `ws2_32.dll`, and `advapi32.dll`.
* **Where it is used**:
  * `src-tauri/src/windows/iphlpapi.rs`.
* **What would happen if removed**: NetBoost would be blind to Windows network telemetry and unable to inspect active sockets or detect streaming apps.
* **Alternatives**: `windows` crate (higher level, but slower compilation and heavier metadata).

---

### 2.4 `reqwest` (v0.12, features: `stream`, `json`, `rustls-tls`)
* **What it is**: A high-performance, asynchronous HTTP client for Rust.
* **Problem it solves**: Writing raw TLS 1.3 socket negotiation and HTTP/1.1 parsing from scratch is complex and error-prone.
* **Why NetBoost needs it**: Probes remote download URLs for HTTP `Range` and `Accept-Ranges` support, handles redirects, reads `ETag` and `Last-Modified` headers, and streams chunk payloads.
* **Why `rustls-tls` is used**: NetBoost configures `default-features = false` and enables `rustls-tls` instead of native OpenSSL/Schannel. `rustls` is written in 100% memory-safe Rust and statically linked, ensuring NetBoost has zero external C library dependencies on end-user machines.
* **Where it is used**:
  * `src-tauri/src/downloads/engine.rs`: Probing and chunk streaming.
  * `src-tauri/src/benchmark/engine.rs`: Downloading benchmark payloads.
* **What would happen if removed**: NetBoost would be unable to connect to HTTPS servers or stream download files.
* **Alternatives**: `hyper`, `surf`, `ureq`.

---

### 2.5 `rusqlite` (v0.32, features: `bundled`)
* **What it is**: Ergonomic Rust wrapper around the SQLite relational database engine.
* **Why `bundled` is used**: Compiles SQLite directly from embedded C source into NetBoost's binary. The user does not need to install SQLite on Windows.
* **Why NetBoost needs it**: Persists download tasks, benchmark comparison histories, user-configured streaming rules, and network adapter DNS backups locally in `%APPDATA%\NetBoost\netboost.db`.
* **Where it is used**:
  * `src-tauri/src/storage/db.rs`.
* **What would happen if removed**: All download tasks, benchmark history, and DNS backup configurations would be lost upon restarting the application.
* **Alternatives**: `sled` (key-value), `sqlx` (async, but heavier compile times).

---

### 2.6 `serde` (v1.0) & `serde_json` (v1.0)
* **What it is**: The standard serialization and deserialization framework for Rust.
* **Problem it solves**: Converting strongly typed Rust structs to JSON strings (and vice-versa) across the IPC boundary.
* **How it works internally**: Uses Rust procedural macros (`#[derive(Serialize, Deserialize)]`) to generate efficient serialization code at compile time with zero runtime reflection overhead.
* **Where it is used**:
  * Across all data structs in `downloads/types.rs`, `bandwidth/manager.rs`, `windows/iphlpapi.rs`, `network/health.rs`, and `storage/db.rs`.

---

### 2.7 `sha2` (v0.10)
* **What it is**: Pure Rust implementation of the SHA-2 cryptographic hash functions (SHA-256).
* **Why NetBoost needs it**: Computes cryptographic checksums of completed multi-worker downloads to ensure parallel chunk assembly did not introduce bit corruption.
* **Where it is used**:
  * `src-tauri/src/downloads/engine.rs` (`compute_file_sha256`).

---

## 3. Frontend Dependencies (`package.json`)

### 3.1 `react` (v18.3.1) & `react-dom` (v18.3.1)
* **What it is**: The component-based declarative UI library developed by Meta.
* **Why NetBoost needs it**: Enables modular UI design, reactive state management (speed history, download progress bars, DNS provider tables), and efficient DOM diffing.
* **Where it is used**:
  * All files in `src/components/`, `src/App.tsx`, and `src/main.tsx`.

---

### 3.2 `vite` (v5.4.11) & `@vitejs/plugin-react` (v4.3.4)
* **What it is**: Next-generation frontend build tool and development server.
* **Why NetBoost needs it**: Provides near-instantaneous Hot Module Replacement (HMR) during development and compiles production bundles via Rollup into `dist/` with tree-shaking and minification.
* **Where it is used**:
  * `vite.config.ts`, `package.json`.

---

### 3.3 `@tauri-apps/api` (v2.2.0)
* **What it is**: The official TypeScript client library for Tauri.
* **Why NetBoost needs it**: Provides `invoke()` to call Rust command handlers and `listen()` to receive asynchronous events emitted by the backend (such as `'netboost-download-detected'`).
* **Where it is used**:
  * `src/services/tauriCommands.ts`, `src/components/Downloads/QuickDownloadModal.tsx`.

---

### 3.4 `tailwindcss` (v3.4.16), `postcss`, & `autoprefixer`
* **What it is**: Utility-first CSS framework.
* **Why NetBoost needs it**: Allows rapid styling directly in TSX markup, generates minimal CSS stylesheets containing only the classes actually used, and facilitates custom dark-mode glassmorphic styling.
* **Where it is used**:
  * `tailwind.config.js`, `postcss.config.js`, `src/index.css`.

---

### 3.5 `lucide-react` (v1.16.0)
* **What it is**: Clean, consistent vector icon library for React.
* **Why NetBoost needs it**: Renders lightweight, scalable SVG icons for network adapters, speed gauges, download statuses, and settings controls without font file overhead.
* **Where it is used**:
  * Across all UI components in `src/components/`.

---

## 4. Architectural Relationship Diagram

```
+-------------------------------------------------------------------------------+
|                      Application Runtime Dependency Hierarchy                 |
+-------------------------------------------------------------------------------+
| Presentation Layer:                                                           |
|   React 18 + Lucide Icons + Tailwind CSS                                      |
|                               ^                                               |
|                               | Renders UI components                         |
|   TypeScript + Vite Bundler   |                                               |
|                               v                                               |
|   @tauri-apps/api (core::invoke & event::listen)                              |
+-------------------------------+-----------------------------------------------+
                                | JSON-RPC Serialized IPC Calls
+-------------------------------+-----------------------------------------------+
| Systems Layer:                                                                |
|   Tauri 2.x Core Engine (Rust)                                                |
|       ├── Tokio (Async Tasks, Channels, Timers, HTTP Loopback)                |
|       ├── Reqwest (HTTP/1.1 Range Header Streaming, TLS 1.3)                  |
|       ├── Rusqlite (Embedded C SQLite Driver)                                 |
|       ├── Serde / Serde_Json (Zero-Copy Serialization)                        |
|       └── Windows-Sys (Win32 FFI: GetIfTable2, GetExtendedTcpTable)           |
+-------------------------------+-----------------------------------------------+
                                | Native Windows C Calls
+-------------------------------+-----------------------------------------------+
| Operating System Layer:                                                       |
|   Windows 10 / 11 Kernel (iphlpapi.dll, ws2_32.dll, kernel32.dll)             |
+-------------------------------------------------------------------------------+
```

