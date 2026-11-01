# 01 — Complete Repository Structure: Directory & File Map

---

## 1. Directory Tree Overview

NetBoost is structured as a multi-tier hybrid application composed of a **Rust core application layer** (`src-tauri/`), a **TypeScript/React desktop presentation layer** (`src/`), a **Chromium browser extension** (`extension/`), and **root-level build and configuration artifacts**.

```
C:\NetBoost\
├── .gitignore                               # Git source control ignore patterns
├── BUILD_PROCESS.md                         # Detailed developer build & release instructions
├── index.html                               # HTML entry point for the desktop web view
├── LICENSE                                  # MIT open-source license
├── NetBoost-Browser-Extension.zip           # Packaged browser extension distribution
├── NetBoost-Installer-Setup.exe             # Compiled NSIS Windows installer setup executable
├── package.json                             # Node.js package manifest and script runner definitions
├── package-lock.json                        # Deterministic dependency lockfile for npm
├── postcss.config.js                        # PostCSS plugin pipeline configuration (Tailwind & Autoprefixer)
├── README.md                                # Master repository documentation and project summary
├── tailwind.config.js                       # Tailwind CSS design system and color palette configuration
├── tsconfig.json                            # TypeScript compiler settings for frontend application
├── tsconfig.node.json                       # TypeScript compiler settings for Vite config files
├── vite.config.ts                           # Vite bundler, HMR server, and build pipeline configuration
│
├── docs\                                    # Comprehensive technical documentation suite
│   └── codebase-deep-dive\                  # 16-part deep-dive technical documentation manual
│
├── extension\                               # Companion Chromium Browser Extension (Chrome / Edge / Brave)
│   ├── manifest.json                        # Chrome Manifest V3 extension configuration
│   ├── background.js                        # Service worker: intercepts downloads & connects to 127.0.0.1:52140
│   ├── content.js                           # Content script injected into web pages (Alt-click bypass detection)
│   ├── popup.html                           # Browser action popup UI layout
│   ├── popup.js                             # Browser action popup status checker & toggle logic
│   ├── icon.png                             # Extension icon asset
│   └── icons\                               # Multi-resolution icons for browser toolbar and store
│       ├── icon16.png                       # 16x16 icon
│       ├── icon32.png                       # 32x32 icon
│       ├── icon48.png                       # 48x48 icon
│       └── icon128.png                      # 128x128 icon
│
├── public\                                  # Static frontend assets served directly by Vite
│   ├── favicon.ico                          # Application window favicon
│   ├── icon.png                             # High-resolution application icon
│   └── logo.png                             # NetBoost branded banner logo
│
├── release\                                 # Packaged release binaries and cryptographic signatures
│   ├── NetBoost_1.0.0_x64-setup.exe         # Official release installer
│   ├── NetBoost-Browser-Extension-v1.0.0.zip# Packaged release extension archive
│   ├── NetBoost.exe                         # Standalone portable Windows executable
│   └── SHA256SUMS.txt                       # Cryptographic checksum manifest for verification
│
├── src\                                     # Frontend Presentation Layer (React 18 + TypeScript)
│   ├── App.tsx                              # Root React view container, routing & polling loop
│   ├── index.css                            # Global styles, Tailwind directives & glassmorphism classes
│   ├── main.tsx                             # React DOM bootstrap entry point (renders App into #root)
│   │
│   ├── assets\                              # Static images bundled by Vite
│   │   ├── icon.png                         # High-resolution icon asset
│   │   └── logo.png                         # High-resolution logo asset
│   │
│   ├── components\                          # React UI Component Library
│   │   ├── Header.tsx                       # Top navigation bar, mode toggle, metrics summary
│   │   │
│   │   ├── Applications\                    # Active processes & streaming detection tab
│   │   │   └── ProcessList.tsx              # Windows process table, TCP connections & streaming config
│   │   │
│   │   ├── Benchmark\                       # Network benchmarking suite
│   │   │   └── BenchmarkView.tsx            # Benchmark test runner, latency delta & SQLite history
│   │   │
│   │   ├── Dashboard\                       # Main operational overview tab
│   │   │   ├── ActiveAdapterCard.tsx        # Physical adapter details, MAC address, Wi-Fi link speed
│   │   │   ├── LiveSpeedChart.tsx           # Real-time SVG throughput graph (60s rolling window)
│   │   │   ├── NetworkHealthIndicator.tsx   # Health score gauge (0-100), latency/jitter/loss grades
│   │   │   ├── NetworkOverview.tsx          # Key metrics row: download, upload, ping, jitter, loss
│   │   │   └── OptimizationStatusBar.tsx    # Active policy banner, congestion indicator, streams count
│   │   │
│   │   ├── Dns\                             # DNS optimization tab
│   │   │   └── DnsOptimizer.tsx             # UDP port 53 benchmark, 1-click netsh tuning & restore
│   │   │
│   │   ├── Downloads\                       # Download accelerator tab
│   │   │   ├── DownloadManager.tsx          # Multi-worker download list, pause/resume, chunk bars
│   │   │   └── QuickDownloadModal.tsx       # Pop-up modal when downloads intercepted from browser
│   │   │
│   │   └── Settings\                        # Application configuration modal
│   │       └── SettingsModal.tsx            # Minimize to tray, startup registry toggle, system logs
│   │
│   ├── services\                            # Backend communication layer
│   │   └── tauriCommands.ts                 # Type-safe wrappers around @tauri-apps/api/core invoke
│   │
│   ├── types\                               # TypeScript data models
│   │   └── network.ts                       # Interfaces for metrics, adapters, tasks, benchmarks, DNS
│   │
│   └── utils\                               # Pure helper functions
│       └── formatters.ts                    # Speed (Mbps), bytes (MB/GB), latency (ms), and ETA formatters
│
└── src-tauri\                               # Backend Systems Layer (Rust + Tauri Core)
    ├── .gitignore                           # Rust target/ directory ignore patterns
    ├── build.rs                             # Cargo build script: links Windows PE icons & manifest
    ├── Cargo.toml                           # Rust package manifest, dependencies & compilation profiles
    ├── Cargo.lock                           # Exact lockfile for all Rust crate dependencies
    ├── tauri.conf.json                      # Tauri desktop configuration: window size, icons, bundle settings
    │
    ├── capabilities\                        # Tauri v2 security capability definitions
    │   └── default.json                     # Permissions granted to the webview window (core:default)
    │
    ├── icons\                               # Native Windows application icons (.ico, .png, .icns)
    │   ├── icon.ico                         # Windows PE binary icon
    │   ├── icon.png                         # Application icon
    │   └── ...                              # Windows Store & Tile logo assets
    │
    ├── src\                                 # Rust Source Code
    │   ├── main.rs                          # Binary entry point: panic hook & #![windows_subsystem = "windows"]
    │   ├── lib.rs                           # Library root: Tauri builder, command handlers, tray, interceptor
    │   ├── logging.rs                       # File logger writing to %APPDATA%\NetBoost\logs\netboost.log
    │   │
    │   ├── bandwidth\                       # Bandwidth management & pacing engine
    │   │   ├── mod.rs                       # Module export definition
    │   │   └── manager.rs                   # BandwidthManager: sliding window median, hysteresis, pacing delay
    │   │
    │   ├── benchmark\                       # Network benchmarking engine
    │   │   ├── mod.rs                       # Module export definition
    │   │   └── engine.rs                    # BenchmarkRunner: idle probes, loaded throughput, comparison logic
    │   │
    │   ├── commands\                        # Tauri IPC command handlers
    │   │   ├── mod.rs                       # Module export definition
    │   │   ├── benchmark.rs                 # run_benchmark, get_benchmark_history, get_benchmark_comparison
    │   │   ├── dns.rs                       # benchmark_dns, apply_dns_provider, restore_original_dns
    │   │   ├── downloads.rs                 # start_download, pause, resume, cancel, retry, open_file
    │   │   ├── network.rs                   # get_realtime_metrics, get_adapters, get_processes, AppState
    │   │   ├── settings.rs                  # get_app_settings, save_app_settings, registry startup
    │   │   └── streaming.rs                 # get_streaming_config, add_user_app, remove_user_app
    │   │
    │   ├── dns\                             # DNS probing and Windows adapter configuration
    │   │   ├── mod.rs                       # Module export definition
    │   │   ├── bench.rs                     # DnsBenchmarkEngine: constructs RFC 1035 UDP queries, probes DNS
    │   │   └── windows.rs                   # WindowsDnsManager: runs netsh, backs up adapter DNS, restores DHCP
    │   │
    │   ├── downloads\                       # Multi-threaded download acceleration engine
    │   │   ├── mod.rs                       # Module export definition
    │   │   ├── engine.rs                    # DownloadEngine: URL probing, Range workers, sparse files, SHA-256
    │   │   ├── interceptor.rs               # Local HTTP IPC server (127.0.0.1:52140) for browser extension
    │   │   └── types.rs                     # Rust structs: DownloadTask, DownloadSegment, Status enums
    │   │
    │   ├── network\                         # Network telemetry and diagnostics
    │   │   ├── mod.rs                       # Module export definition
    │   │   ├── adapter.rs                   # AdapterTracker: polls GetIfTable2, calculates throughput rate
    │   │   ├── health.rs                    # evaluate_network_health: calculates 0-100 score, grades A+ to D
    │   │   └── ping.rs                      # LatencyDiagnostics: TCP connect RTT, RFC 3550 jitter algorithm
    │   │
    │   ├── storage\                         # SQLite persistent database layer
    │   │   ├── mod.rs                       # Module export definition
    │   │   └── db.rs                        # Database: rusqlite wrapper, migrations, tasks, benchmarks, settings
    │   │
    │   ├── streaming\                       # Media streaming heuristic detection
    │   │   ├── mod.rs                       # Module export definition
    │   │   └── detector.rs                  # StreamingDetector: classifies processes, HTTPS sockets, user rules
    │   │
    │   └── windows\                         # Native Windows API FFI integrations
    │       ├── mod.rs                       # Module export definition
    │       └── iphlpapi.rs                  # Win32 FFI: GetIfTable2, GetExtendedTcpTable, GetProcessIoCounters
    │
    └── tests\                               # Rust Integration Test Harnesses
        ├── test_app_monitor.rs              # Tests Win32 process enumeration & TCP table parsing
        ├── test_bandwidth_coordinator.rs    # Tests AIMD congestion hysteresis & pacing adaptations
        ├── test_benchmark_engine.rs         # Tests statistical median latency & throughput benchmarks
        ├── test_dns_optimizer.rs            # Tests wire-format RFC 1035 DNS packet generation & parsing
        ├── test_download_engine.rs          # Tests multi-worker HTTP range downloads, pause/resume, SHA-256
        ├── test_network.rs                  # Tests hardware interface querying & throughput delta math
        ├── test_production_persistence.rs   # Tests SQLite table creation, recovery & CRUD operations
        └── test_streaming_heuristics.rs     # Tests process classification rules for YouTube, Netflix, Discord
```

---

## 2. Directory Responsibilities and Layer Boundaries

### 2.1 The Root Workspace Directory (`C:\NetBoost\`)
* **Role**: Orchestrates the multi-lingual build pipeline, dependency management, and top-level distribution files.
* **Key Artifacts**:
  * `package.json`: Defines frontend build scripts (`npm run dev`, `npm run build`, `npm run tauri`).
  * `vite.config.ts`: Configures Vite to compile TypeScript and bundle assets to `dist/`, which Tauri embeds into the executable.
  * `tailwind.config.js`: Defines custom color definitions (`accent-cyan`, `dark-900`) and font stacks.
  * `BUILD_PROCESS.md`: Explains step-by-step instructions for MSVC compiler linking, resource compilation, and NSIS installer generation.

### 2.2 The Browser Extension Layer (`extension/`)
* **Role**: Integrates Chromium-based browsers (Chrome, Edge, Brave, Opera) into NetBoost's acceleration pipeline.
* **Key Artifacts**:
  * `manifest.json`: Manifest V3 specification declaring permissions (`downloads`, `cookies`, `storage`).
  * `background.js`: Service worker listening on `chrome.downloads.onCreated`. Forwards candidate downloads to NetBoost's local loopback server (`http://127.0.0.1:52140/intercept`).
  * `content.js`: Injected into all web pages to listen for `Alt`-clicks, allowing users to instantly bypass NetBoost without opening the extension menu.

### 2.3 The Frontend Presentation Layer (`src/`)
* **Role**: High-performance, reactive desktop user interface rendered via Windows WebView2.
* **Component Subdivisions**:
  * `components/Dashboard/`: Displays real-time hardware telemetry, live throughput chart, health indicators, and active network policy.
  * `components/Applications/`: Live inspection of Windows processes with open TCP connections, highlighting streaming applications and allowing custom app configuration.
  * `components/Downloads/`: Download manager interface with segment progress bars, speed gauges, pause/resume buttons, and file opening actions.
  * `components/Dns/`: Interactive benchmark of Cloudflare, Google, Quad9, and OpenDNS resolvers with one-click `netsh` system application and restoration.
  * `components/Benchmark/`: Controlled network load benchmark comparing unoptimized vs. streaming-priority performance.
  * `components/Settings/`: Application settings, Windows startup registry configuration, and live engine log viewer.
  * `services/tauriCommands.ts`: Bridge module translating between React TypeScript functions and Tauri's Rust IPC invoke handlers.

### 2.4 The Backend Core Engine (`src-tauri/`)
* **Role**: Native Windows service engine running natively in compiled machine code with zero garbage collection overhead.
* **Module Subdivisions**:
  * `src/windows/`: Directly interacts with Windows kernel drivers via `windows-sys` FFI (`iphlpapi.dll`).
  * `src/bandwidth/`: The intelligence engine. Uses sliding-window medians and hysteresis to identify bufferbloat and adapt pacing delays.
  * `src/downloads/`: Multi-threaded HTTP Range worker orchestration using Tokio async channels and sparse file writing.
  * `src/dns/`: Constructs raw RFC 1035 UDP packets and automates Windows adapter DNS via `netsh`.
  * `src/storage/`: SQLite database persistence layer managing tasks, benchmarks, and settings with crash recovery.
  * `src/commands/`: The 22 Tauri IPC endpoints exposed to the frontend webview.

---

## 3. Complete Source File Catalog

| File Path | Language | Primary Responsibility |
| :--- | :--- | :--- |
| `src-tauri/src/main.rs` | Rust | Program entry point. Configures panic hooks and suppresses Windows CMD window (`#![windows_subsystem = "windows"]`). |
| `src-tauri/src/lib.rs` | Rust | Tauri application builder. Registers 22 IPC commands, builds system tray, initializes background loopback server. |
| `src-tauri/src/logging.rs` | Rust | Thread-safe file logger persisting operational events to `%APPDATA%\NetBoost\logs\netboost.log`. |
| `src-tauri/src/bandwidth/manager.rs` | Rust | Congestion detection, sliding window median calculation, hysteresis counters, and read pacing delay calculation. |
| `src-tauri/src/benchmark/engine.rs` | Rust | Orchestrates controlled network benchmarks, baseline comparisons, and statistical probe sampling. |
| `src-tauri/src/commands/benchmark.rs` | Rust | Tauri command handlers for running benchmarks, clearing history, and fetching comparison records. |
| `src-tauri/src/commands/dns.rs` | Rust | Tauri command handlers for DNS benchmarking, applying static DNS, and restoring original adapter settings. |
| `src-tauri/src/commands/downloads.rs` | Rust | Tauri command handlers for download lifecycle (start, pause, resume, cancel, retry, file discovery). |
| `src-tauri/src/commands/network.rs` | Rust | Central application state (`AppState`), telemetry polling command (`get_realtime_metrics`), and mode toggles. |
| `src-tauri/src/commands/settings.rs` | Rust | Settings persistence and Windows Registry auto-start configuration (`HKCU\Software\...\Run`). |
| `src-tauri/src/commands/streaming.rs` | Rust | User streaming application configuration commands (add/remove custom binary names). |
| `src-tauri/src/dns/bench.rs` | Rust | Low-level RFC 1035 DNS wire-format query builder and UDP socket prober for public and local resolvers. |
| `src-tauri/src/dns/windows.rs` | Rust | Windows `netsh` automation, adapter DNS configuration extraction, backup generation, and restoration. |
| `src-tauri/src/downloads/engine.rs` | Rust | Core parallel download engine: HTTP range probe, sparse `.part` file pre-allocation, worker loop, and SHA-256. |
| `src-tauri/src/downloads/interceptor.rs` | Rust | Local HTTP loopback server listening on `127.0.0.1:52140` to exchange messages with the browser extension. |
| `src-tauri/src/downloads/types.rs` | Rust | Data transfer structs and serialization definitions for downloads and segment states. |
| `src-tauri/src/network/adapter.rs` | Rust | Hardware network adapter tracking, speed delta calculation over time, and active adapter selection. |
| `src-tauri/src/network/health.rs` | Rust | Algorithmic network health scoring (0–100) based on RTT, RFC 3550 jitter, and packet loss percentage. |
| `src-tauri/src/network/ping.rs` | Rust | TCP connect latency prober and smoothed jitter variance calculation against anycast DNS resolvers. |
| `src-tauri/src/storage/db.rs` | Rust | Embedded SQLite database manager, table schemas, indexes, CRUD methods, and startup crash recovery. |
| `src-tauri/src/streaming/detector.rs` | Rust | Streaming process classifier: evaluates executable names, active HTTPS socket counts, and I/O rates. |
| `src-tauri/src/windows/iphlpapi.rs` | Rust | Win32 FFI wrappers for `GetIfTable2`, `GetExtendedTcpTable`, and `GetProcessIoCounters`. |
| `src/main.tsx` | TypeScript | Frontend React entry point. Bootstraps DOM root and mounts the `App` component. |
| `src/App.tsx` | TypeScript | Top-level React container. Manages tab state, telemetry polling interval, global link interception, modals. |
| `src/index.css` | CSS | Tailwind CSS directives, dark mode styling, custom scrollbars, and glassmorphism definitions. |
| `src/services/tauriCommands.ts` | TypeScript | Bridge layer providing type-safe async functions wrapping Tauri's `invoke` API with browser fallbacks. |
| `src/types/network.ts` | TypeScript | TypeScript interfaces matching Rust serde serialization for all telemetry, download, and config objects. |
| `src/utils/formatters.ts` | TypeScript | Pure formatting utilities for network speeds (bps to Gbps), bytes (B to GB), latency (ms), and ETAs. |
| `src/components/Header.tsx` | TypeScript | Desktop header with branding, tab navigation buttons, operating mode selector, and settings button. |
| `src/components/Applications/ProcessList.tsx` | TypeScript | Real-time table of Windows processes using network sockets, with streaming tags and custom app config drawer. |
| `src/components/Benchmark/BenchmarkView.tsx` | TypeScript | Network benchmark runner UI, scenario selector, performance delta graphs, and SQLite run history. |
| `src/components/Dashboard/ActiveAdapterCard.tsx` | TypeScript | Card showing physical adapter name, PHY link speed, IP, Gateway, MAC address, and Wi-Fi signal. |
| `src/components/Dashboard/LiveSpeedChart.tsx` | TypeScript | Pure SVG responsive graph showing a 60-second rolling window of upload and download throughput. |
| `src/components/Dashboard/NetworkHealthIndicator.tsx` | TypeScript | Health gauge with letter grades for latency, jitter, and stability, with explanatory diagnosis factors. |
| `src/components/Dashboard/NetworkOverview.tsx` | TypeScript | Metric cards for download speed, upload speed, latency (ping), jitter, packet loss, and session total. |
| `src/components/Dashboard/OptimizationStatusBar.tsx` | TypeScript | Status banner indicating active policy, bufferbloat warning status, active streams, and active downloads. |
| `src/components/Dns/DnsOptimizer.tsx` | TypeScript | DNS benchmark UI displaying latency comparisons, recommended provider badges, and 1-click apply/restore. |
| `src/components/Downloads/DownloadManager.tsx` | TypeScript | Download accelerator interface with URL input, concurrency selector, task cards, and segment visualizers. |
| `src/components/Downloads/QuickDownloadModal.tsx` | TypeScript | Foreground modal triggered when browser extension intercepts a download, prompting user choice. |
| `src/components/Settings/SettingsModal.tsx` | TypeScript | Preferences dialog for minimize-to-tray, startup registry key, default operating mode, and logs viewer. |
| `extension/manifest.json` | JSON | Chrome Manifest V3 manifest declaring background service worker, permissions, and icons. |
| `extension/background.js` | JavaScript | Extension service worker capturing downloads and communicating with NetBoost core via local loopback. |
| `extension/content.js` | JavaScript | Web page content script detecting `Alt`-click on download links to trigger direct browser bypass. |
| `extension/popup.html` | HTML | Browser toolbar popup layout displaying connection status and download capture toggle. |
| `extension/popup.js` | JavaScript | Popup script checking whether NetBoost desktop engine is running on `127.0.0.1:52140`. |

