# 02 — Programming Language Fundamentals

---

## Introduction

NetBoost is built using two distinct programming language ecosystems that work in harmony:
1. **Rust (Systems Programming Layer)**: High-performance, zero-cost abstractions, deterministic memory management without a garbage collector, and direct access to Windows Win32 C APIs.
2. **TypeScript / JavaScript (Presentation & Extension Layer)**: Expressive, component-driven, statically typed UI code running inside a Chromium-based webview and a browser background service worker.

This document teaches the fundamentals of both languages from the ground up, contrasting their paradigms and highlighting exactly how their language features are utilized in NetBoost.

---

# PART I: RUST FUNDAMENTALS

## 1. What is Rust and Why Does NetBoost Use It?
Rust is a statically compiled systems programming language developed to provide the raw speed and hardware-level control of C/C++, but with guaranteed memory safety and thread safety enforced at compile time.

### Why NetBoost Uses Rust
* **Deterministic Low Latency**: Unlike C# (.NET), Go, or Java, Rust has **no garbage collector (GC)**. In a networking utility managing high-throughput packet transfers and measuring millisecond-level jitter, a garbage collection pause of 15 ms would cause false congestion reports and destroy pacing precision.
* **Direct OS Kernel Interaction**: NetBoost must invoke raw Windows C functions (`GetIfTable2`, `GetExtendedTcpTable`, `GetProcessIoCounters`) in `iphlpapi.dll`. Rust provides native C Foreign Function Interface (FFI) with zero overhead.
* **Fearless Concurrency**: Managing 12 concurrent download workers while simultaneously probing ping, parsing TCP sockets, and serving local HTTP requests requires bulletproof multi-threading. Rust's compiler guarantees data-race freedom.

---

## 2. Compilation and Runtime Model

### How Compilation Works
Rust code (`.rs` files) is compiled ahead-of-time (AOT) by `rustc` using the LLVM compiler infrastructure into native machine code (`.exe` on Windows).

```
+---------------+      +---------+      +-------------------+      +-----------------+
| Rust Code     | ---> | rustc   | ---> | LLVM Optimization | ---> | Native Windows  |
| (*.rs source) |      | (HIR/   |      | & Code Generation |      | PE Binary       |
|               |      |  MIR)   |      |                   |      | (NetBoost.exe)  |
+---------------+      +---------+      +-------------------+      +-----------------+
```

### The Runtime
Rust has **almost no runtime**. There is no virtual machine (like the JVM or CLR). The binary executes directly on the CPU. The only runtime code included is a tiny initialization stub that sets up the stack and calls `main()`, plus whatever libraries the application imports (such as the Tokio asynchronous runtime).

---

## 3. Memory Management: Ownership, Borrowing, and Lifetimes

Rust achieves memory safety without a garbage collector using a compile-time system called **Ownership**.

### The Three Rules of Ownership
1. Each value in Rust has an **owner** (a variable).
2. There can only be **one owner at a time**.
3. When the owner goes out of scope, the value is **automatically dropped** (memory is deallocated immediately).

### Independent Example
```rust
fn main() {
    let s1 = String::from("NetBoost"); // s1 owns the allocated string
    let s2 = s1;                       // Ownership MOVES to s2! s1 is now invalid.
    // println!("{}", s1);             // COMPILE ERROR: value borrowed here after move
    println!("{}", s2);                // Works perfectly
} // s2 goes out of scope here; memory is freed instantly.
```

### How It Appears in NetBoost
In `src-tauri/src/downloads/engine.rs`:
```rust
pub async fn start_download(
    &self,
    url: &str,                         // Borrowed reference (does not take ownership)
    custom_filename: Option<String>,   // Takes ownership of String if provided
    concurrency: Option<usize>,
    bandwidth_mgr: Arc<Mutex<BandwidthManager>>, // Cloned atomic reference count
) -> Result<String, String> { ... }
```

### Why Developers Used It Here
In network download engines, buffer copies and memory leaks cause catastrophic performance degradation. Rust's ownership guarantees that 64 KB chunk buffers received over TCP sockets are freed the instant they are written to disk, preventing memory bloat.

### What Would Happen If Implemented Differently?
* In C/C++, developers would have to manually call `malloc` and `free`. A single missing `free` inside an interrupted download loop would leak hundreds of megabytes per minute.
* In Go or C#, the garbage collector would periodically pause execution to sweep orphaned socket buffers, causing latency spikes in the ping diagnostic engine.

---

## 4. Borrowing and References (`&` and `&mut`)

Instead of moving ownership, a function can **borrow** a value using references.
* `&T`: Immutable borrow (read-only). You can have **any number** of immutable references at the same time.
* `&mut T`: Mutable borrow (read-write). You can have **only one** mutable reference at a time, and no immutable references simultaneously.

### Independent Example
```rust
fn calculate_length(s: &String) -> usize { // Borrows s read-only
    s.len()
}

fn append_status(s: &mut String) {        // Borrows s mutably
    s.push_str(" [Active]");
}
```

### How It Appears in NetBoost
In `src-tauri/src/bandwidth/manager.rs`:
```rust
// Immutable reference: read-only access to calculate median without changing state
pub fn calculate_median_latency(&self) -> f64 { ... }

// Mutable reference: modifies internal window and counters
pub fn evaluate_congestion_with_traffic(
    &mut self,
    current_latency_ms: f64,
    current_jitter_ms: f64,
    active_downloads: u32,
    netboost_speed_bps: f64,
    active_streaming_sessions: u32,
) -> (bool, Option<String>) { ... }
```

---

## 5. Structs and Methods (Classes vs. Structs)

Rust does not have `class` or classical inheritance. Instead, it separates data from behavior:
* **`struct`**: Defines the data fields and layout in memory.
* **`impl` block**: Defines functions and methods associated with that struct.

### Independent Example
```rust
struct LatencySample {
    rtt_ms: f64,
    jitter_ms: f64,
}

impl LatencySample {
    // Associated constructor function
    fn new(rtt: f64, jitter: f64) -> Self {
        Self { rtt_ms: rtt, jitter_ms: jitter }
    }

    // Method taking immutable reference to self
    fn is_high_latency(&self) -> bool {
        self.rtt_ms > 100.0
    }
}
```

### How It Appears in NetBoost
In `src-tauri/src/bandwidth/manager.rs`:
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

impl BandwidthManager {
    pub fn new() -> Self { ... }
    pub fn set_mode(&mut self, mode: OperatingMode) { ... }
    pub fn get_pacing_delay_ms(&self) -> u64 { ... }
}
```

---

## 6. Enums and Pattern Matching

Rust enums are **algebraic data types** (sum types). Unlike C or Java enums, Rust enums can carry rich data in each variant.

### Independent Example
```rust
enum DownloadStatus {
    Queued,
    Downloading { speed_bps: f64, progress_pct: f64 },
    Failed(String),
}

fn describe_status(status: &DownloadStatus) -> String {
    match status {
        DownloadStatus::Queued => "Waiting in queue...".to_string(),
        DownloadStatus::Downloading { speed_bps, progress_pct } => {
            format!("Running at {:.1} Mbps ({}%)", speed_bps / 1_000_000.0, progress_pct)
        }
        DownloadStatus::Failed(err) => format!("Error: {}", err),
    }
}
```

### How It Appears in NetBoost
In `src-tauri/src/bandwidth/manager.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingMode {
    Streaming,
    Balanced,
    Download,
}
```
And in pattern matching:
```rust
match self.mode {
    OperatingMode::Streaming => {
        self.current_pacing_delay_ms = if active_streaming_sessions > 0 { 45 } else { 35 };
        self.concurrency_throttle_factor = 0.40;
    }
    OperatingMode::Balanced => {
        self.current_pacing_delay_ms = if active_streaming_sessions > 0 { 20 } else { 12 };
        self.concurrency_throttle_factor = 0.70;
    }
    OperatingMode::Download => {
        self.current_pacing_delay_ms = 0;
        self.concurrency_throttle_factor = 1.0;
    }
}
```

---

## 7. Error Handling: `Result<T, E>` and `Option<T>`

Rust has **no exceptions** (`try / catch / throw`). Instead, all operations that might fail or return nothing return explicit types:
* **`Option<T>`**: Either `Some(T)` or `None`.
* **`Result<T, E>`**: Either `Ok(T)` or `Err(E)`.

The `?` operator unwraps `Ok` values or immediately returns early with the `Err`.

### Independent Example
```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}

fn calculate() -> Result<(), String> {
    let result = divide(10.0, 2.0)?; // Unwraps 5.0 or returns early if Err
    println!("Result: {}", result);
    Ok(())
}
```

### How It Appears in NetBoost
In `src-tauri/src/dns/windows.rs`:
```rust
pub fn apply_dns(
    db: &Database,
    iface: &str,
    primary_ip: &str,
    secondary_ip: Option<&str>,
) -> Result<(), String> {
    // If backup fails, return Err immediately using '?'
    let _ = Self::backup_dns_if_needed(db, iface)?;

    run_netsh_cmd(&[
        "interface", "ipv4", "set", "dnsservers",
        &format!("name={}", iface),
        "static", primary_ip, "primary",
    ])?;

    Ok(())
}
```

---

## 8. Multi-Threading & Concurrency Primitives

Because multiple threads in NetBoost access shared data (such as the database, download engine, and bandwidth coordinator), Rust provides thread-safe concurrency containers:

### 8.1 `Arc<T>` (Atomic Reference Counting)
`Arc` allows multiple threads to own shared, read-only access to the same heap allocation. When an `Arc` is cloned, only the reference counter is incremented.

### 8.2 `Mutex<T>` (Mutual Exclusion Lock)
`Mutex` provides exclusive mutable access to wrapped data. To read or write, a thread must call `.lock().unwrap()`. Rust's `Mutex` wraps the *data itself*, so it is impossible to access data without holding the lock.

### 8.3 `RwLock<T>` (Reader-Writer Lock)
`RwLock` allows multiple readers simultaneously, but only one writer at a time. Used in `StreamingDetector` where processes frequently read configured apps without locking out each other.

### 8.4 `AtomicBool`
Lock-free, thread-safe boolean flags updated using hardware atomic CPU instructions (`Ordering::Relaxed`). Used in `ActiveDownloadControl` to signal pause state across download threads instantly.

### Independent Example
```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..5 {
        let counter_clone = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter_clone.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for h in handles { h.join().unwrap(); }
    println!("Final count: {}", *counter.lock().unwrap()); // 5
}
```

### How It Appears in NetBoost
In `src-tauri/src/commands/network.rs`:
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

---

## 9. Asynchronous Programming with Tokio

While standard threads (`std::thread`) map 1:1 to OS kernel threads (heavyweight, ~1MB stack), **Tokio tasks** are lightweight green threads scheduled by a work-stealing thread pool. A single NetBoost process can run dozens of parallel download range workers with minimal memory overhead.

### Keywords: `async` and `.await`
* `async fn`: Returns a `Future` (a state machine representing deferred execution).
* `.await`: Yields execution back to the Tokio event loop until the I/O event (socket read, file write, timer) completes.

### Independent Example
```rust
async fn fetch_header() -> Result<u64, reqwest::Error> {
    let resp = reqwest::get("https://example.com").await?;
    Ok(resp.content_length().unwrap_or(0))
}
```

### How It Appears in NetBoost
In `src-tauri/src/downloads/engine.rs`:
```rust
// Worker loop reading chunks asynchronously
tokio::select! {
    _ = cancel_rx.recv() => {
        break;
    }
    chunk_res = resp.chunk() => {
        match chunk_res {
            Ok(Some(chunk)) => {
                file.write_all(&chunk).await.map_err(|e| format!("Write failed: {}", e))?;
                // Pacing delay sleep yields back to Tokio without blocking other tasks!
                if pacing_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(pacing_delay_ms)).await;
                }
            }
            ...
        }
    }
}
```

---

## 10. Win32 Foreign Function Interface (FFI) and `unsafe`

Because Rust cannot guarantee safety when calling external C libraries compiled outside its type system, foreign C calls must be enclosed in an `unsafe` block.

`unsafe` does NOT disable the type checker or turn off borrow rules. It simply tells the compiler: *"I, the programmer, have verified that the memory pointers passed to this external C function satisfy the operating system's requirements."*

### How It Appears in NetBoost
In `src-tauri/src/windows/iphlpapi.rs`:
```rust
pub fn query_interface_table() -> Vec<RawAdapterInfo> {
    let mut adapters = Vec::new();
    unsafe {
        let mut table_ptr: *mut MIB_IF_TABLE2 = null_mut();
        // Invoke native Windows GetIfTable2 in iphlpapi.dll
        let res = GetIfTable2(&mut table_ptr);

        if res == NO_ERROR && !table_ptr.is_null() {
            let table = &*table_ptr;
            let slice = std::slice::from_raw_parts(table.Table.as_ptr(), table.NumEntries as usize);

            for row in slice {
                // Safely extract hardware statistics from raw C struct
                let is_up = row.OperStatus == 1;
                ...
            }
            // Free the memory allocated by the Windows kernel
            FreeMibTable(table_ptr as *const c_void);
        }
    }
    adapters
}
```

---

# PART II: TYPESCRIPT & JAVASCRIPT FUNDAMENTALS

## 1. What is TypeScript and Why Does NetBoost Use It?
TypeScript is a strongly typed superset of JavaScript developed by Microsoft. It adds static type annotations, interfaces, generics, and compile-time verification to JavaScript code.

### Why NetBoost Uses TypeScript
* **Contract Synchronization**: NetBoost passes complex network metrics, process tables, and download task arrays across the Tauri IPC bridge. TypeScript interfaces ensure the frontend data structures match the Rust backend's Serde JSON outputs byte-for-byte.
* **Component Prop Validation**: React components (`LiveSpeedChart`, `NetworkHealthIndicator`, `ProcessList`) have strictly defined props, preventing runtime `undefined` errors.

---

## 2. Compilation and Runtime Model

### How Compilation Works
TypeScript files (`.ts`, `.tsx`) cannot be executed directly by web browsers. 
1. **Type Checking**: The TypeScript compiler (`tsc`) verifies types across `src/` according to `tsconfig.json`.
2. **Bundling & Transpilation**: **Vite** uses `esbuild` and `@vitejs/plugin-react` to strip types and compile modern JSX syntax into highly optimized JavaScript ES2020 bundles stored in `dist/`.
3. **Execution**: The compiled bundle is loaded by Tauri inside a native Windows **WebView2** control (powered by the Microsoft Edge Chromium engine).

```
+--------------------+      +-----------+      +-------------------+      +--------------------+
| TypeScript / React | ---> | tsc       | ---> | Vite (esbuild)    | ---> | Windows WebView2   |
| (*.tsx source)     |      | Typecheck |      | Bundle to dist/   |      | (Chromium Engine)  |
+--------------------+      +-----------+      +-------------------+      +--------------------+
```

---

## 3. Interfaces and Type Annotations

TypeScript defines structural contracts using `interface` and `type`.

### Discriminated Unions
A discriminated union uses a single common literal property (a "tag") to distinguish between different variants of a type.

### How It Appears in NetBoost
In `src/types/network.ts`:
```typescript
export type OperatingMode = 'streaming' | 'balanced' | 'download';

export type SegmentStatus = 'pending' | 'downloading' | 'paused' | 'completed' | 'failed';

export interface DownloadSegment {
  index: number;
  startByte: number;
  endByte: number;
  downloadedBytes: number;
  speedBps: number;
  status: SegmentStatus;
}

export interface NetworkHealth {
  score: 'excellent' | 'good' | 'moderate' | 'poor';
  scoreNumber: number; // 0 - 100
  reasons: string[];
  latencyGrade: string;
  jitterGrade: string;
  stabilityGrade: string;
}
```

---

## 4. Asynchronous JavaScript: Promises and the Event Loop

Unlike Rust's multi-threaded Tokio runtime, JavaScript in the browser runs on a **single-threaded Event Loop**. 

Asynchronous operations (fetching data, waiting for timers, invoking Tauri IPC commands) do not block the UI thread. Instead, they register callbacks or return a **Promise** that resolves when the background task finishes.

### Independent Example
```typescript
async function fetchMetrics(): Promise<number> {
  const response = await fetch('/api/speed');
  const data = await response.json();
  return data.speedBps;
}
```

### How It Appears in NetBoost
In `src/services/tauriCommands.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core';
import { RealtimeMetrics } from '../types/network';

export async function getRealtimeMetrics(): Promise<RealtimeMetrics> {
  if (isTauriEnvironment()) {
    // Asynchronously invokes Rust command handler across IPC bridge
    return await invoke<RealtimeMetrics>('get_realtime_metrics');
  }
  return generatePreviewMetrics();
}
```

---

## 5. React 18 Paradigm: Components and Hooks

React represents user interfaces as pure functions of state. When state changes, React recalculates the virtual DOM and applies minimal patches to the real DOM.

### 5.1 `useState`
Manages reactive state variables inside a functional component.

### 5.2 `useEffect`
Executes side effects (timers, polling loops, event listeners) after rendering.

### 5.3 `useMemo`
Caches expensive calculations (such as generating SVG path strings for 60 data points) so they only re-run when dependencies change.

### How It Appears in NetBoost
In `src/App.tsx`:
```typescript
export const App: React.FC = () => {
  const [currentTab, setCurrentTab] = useState<'dashboard' | 'applications' | 'downloads' | 'dns' | 'benchmark'>('dashboard');
  const [metrics, setMetrics] = useState<RealtimeMetrics | null>(null);

  // Polling loop for live telemetry running once every 1000ms
  useEffect(() => {
    let mounted = true;
    const poll = async () => {
      try {
        const [m, opt] = await Promise.all([
          getRealtimeMetrics(),
          getOptimizationState()
        ]);
        if (!mounted) return;
        setMetrics(m);
      } catch (err) {
        console.error('Error fetching live telemetry:', err);
      }
    };

    poll();
    const interval = setInterval(poll, 1000);
    return () => {
      mounted = false;
      clearInterval(interval); // Clean up interval when unmounted
    };
  }, []);
  ...
}
```

---

# PART III: BROWSER EXTENSION JAVASCRIPT (MANIFEST V3)

The browser companion extension (`extension/background.js`) runs in a separate execution environment: **Chromium Service Workers**.

### Key Rules of Manifest V3:
1. **No Persistent Background Pages**: Service workers are event-driven and can be suspended by Chrome when idle.
2. **Synchronous Interception Guarantee**: To prevent breaking native browser downloads, the extension cannot use blocking webRequest APIs. Instead, it listens to `chrome.downloads.onCreated`.
3. **Local Loopback Communication**: The service worker sends standard HTTP `POST` requests to `http://127.0.0.1:52140` to exchange data with the NetBoost desktop application.

```javascript
// extension/background.js
chrome.downloads.onCreated.addListener((downloadItem) => {
  // Returns immediately to prevent Chrome's "Save As" window from freezing
  handleDownloadCreated(downloadItem);
});
```

---

# PART IV: LANGUAGE COMPARISON MATRIX

| Dimension | Rust Backend (`src-tauri/`) | TypeScript Frontend (`src/`) |
| :--- | :--- | :--- |
| **Execution Model** | Compiled AOT native machine code | Interpreted / JIT compiled in Chromium WebView2 |
| **Memory Model** | Compile-time Ownership & Borrowing | Garbage Collector (V8 Engine) |
| **Concurrency** | Multi-threaded OS threads & Tokio tasks | Single-threaded Event Loop with Promises |
| **Data Safety** | Data-race free, Null-pointer free | Type-checked at build time, potential runtime nulls |
| **Error Handling** | `Result<T, E>` & `Option<T>` with `?` | `try / catch` & Promise rejections |
| **OS Access** | Direct Win32 C FFI pointers | Sandboxed; mediated strictly via Tauri IPC commands |

