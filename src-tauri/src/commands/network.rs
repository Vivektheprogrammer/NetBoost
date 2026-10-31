use crate::bandwidth::manager::{BandwidthManager, OperatingMode, OptimizationState};
use crate::benchmark::engine::BenchmarkRunner;
use crate::downloads::engine::DownloadEngine;
use crate::logging::NetBoostLogger;
use crate::network::adapter::{AdapterTracker, NetworkAdapter};
use crate::network::health::{evaluate_network_health, NetworkHealthReport};
use crate::network::ping::LatencyDiagnostics;
use crate::storage::db::Database;
use crate::streaming::detector::StreamingDetector;
use crate::windows::iphlpapi::{ProcessNetworkMonitor, ProcessNetworkReport};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetrics {
    #[serde(rename = "downloadSpeedBps")]
    pub download_speed_bps: f64,
    #[serde(rename = "uploadSpeedBps")]
    pub upload_speed_bps: f64,
    #[serde(rename = "totalBytesReceived")]
    pub total_bytes_received: u64,
    #[serde(rename = "totalBytesSent")]
    pub total_bytes_sent: u64,
    #[serde(rename = "latencyMs")]
    pub latency_ms: f64,
    #[serde(rename = "jitterMs")]
    pub jitter_ms: f64,
    #[serde(rename = "packetLossPercent")]
    pub packet_loss_percent: f64,
    #[serde(rename = "activeAdapter")]
    pub active_adapter: Option<NetworkAdapter>,
    pub health: NetworkHealthReport,
    pub timestamp: u64,
}

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

impl AppState {
    pub fn new() -> Self {
        let logger = Arc::new(NetBoostLogger::init());

        // Initialize SQLite database
        let db_path = if let Ok(app_data) = std::env::var("APPDATA") {
            let p = std::path::PathBuf::from(app_data).join("NetBoost");
            let _ = std::fs::create_dir_all(&p);
            p.join("netboost.db")
        } else {
            std::path::PathBuf::from("netboost.db")
        };

        let db = Arc::new(Database::new(&db_path).unwrap_or_else(|_| Database::in_memory().unwrap()));

        // Automatic startup crash recovery: pause interrupted downloads cleanly
        if let Ok(count) = db.recover_interrupted_downloads() {
            if count > 0 {
                logger.log(
                    "WARN",
                    &format!("Startup crash recovery: reset {} interrupted tasks to paused state.", count),
                );
            }
        }

        let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));
        let dl_engine = Arc::new(DownloadEngine::new(db.clone(), None));
        let detector = Arc::new(StreamingDetector::new());

        // Load configured streaming apps from database
        if let Ok(saved_apps) = db.get_streaming_apps() {
            detector.load_user_apps(saved_apps);
        }

        let bm_runner = Arc::new(BenchmarkRunner::new(db.clone(), b_mgr.clone()));

        Self {
            adapter_tracker: Mutex::new(AdapterTracker::new()),
            latency_diag: Mutex::new(LatencyDiagnostics::new()),
            bandwidth_mgr_arc: b_mgr,
            streaming_detector: detector,
            process_monitor: Mutex::new(ProcessNetworkMonitor::new()),
            download_engine: dl_engine,
            db,
            benchmark_runner: bm_runner,
            logger,
        }
    }
}

#[tauri::command]
pub fn get_realtime_metrics(state: State<'_, AppState>) -> RealtimeMetrics {
    let mut tracker = state.adapter_tracker.lock().unwrap();
    let (adapters, active_adapter, down_bps, up_bps) = tracker.poll_adapters();

    let mut diag = state.latency_diag.lock().unwrap();
    let (lat_ms, jit_ms, loss_pct) = diag.sample_latency();

    let health = evaluate_network_health(lat_ms, jit_ms, loss_pct);

    let (tot_rx, tot_tx) = adapters
        .iter()
        .filter(|a| a.is_up && a.adapter_type != "loopback")
        .fold((0u64, 0u64), |acc, a| (acc.0 + a.bytes_received, acc.1 + a.bytes_sent));

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Collect download activity metrics
    let (active_downloads, netboost_dl_speed) = {
        let tasks = state.download_engine.get_all_tasks();
        let count = tasks
            .iter()
            .filter(|t| t.status == crate::downloads::types::DownloadStatus::Downloading)
            .count() as u32;
        let speed: f64 = tasks
            .iter()
            .filter(|t| t.status == crate::downloads::types::DownloadStatus::Downloading)
            .map(|t| t.speed_bps)
            .sum();
        (count, speed)
    };

    let active_streams = {
        let mut monitor = state.process_monitor.lock().unwrap();
        monitor
            .snapshot_processes_with_detector(Some(&state.streaming_detector))
            .iter()
            .filter(|p| p.is_streaming)
            .count() as u32
    };

    // Update BandwidthManager sliding window & hysteresis state
    {
        let mut mgr = state.bandwidth_mgr_arc.lock().unwrap();
        mgr.record_probe(lat_ms, jit_ms, loss_pct);
        mgr.evaluate_congestion_with_traffic(
            lat_ms,
            jit_ms,
            active_downloads,
            netboost_dl_speed,
            active_streams,
        );
    }

    RealtimeMetrics {
        download_speed_bps: down_bps,
        upload_speed_bps: up_bps,
        total_bytes_received: tot_rx,
        total_bytes_sent: tot_tx,
        latency_ms: lat_ms,
        jitter_ms: jit_ms,
        packet_loss_percent: loss_pct,
        active_adapter,
        health,
        timestamp: now_ms,
    }
}

#[tauri::command]
pub fn get_network_adapters(state: State<'_, AppState>) -> Vec<NetworkAdapter> {
    let mut tracker = state.adapter_tracker.lock().unwrap();
    let (adapters, _, _, _) = tracker.poll_adapters();
    adapters
}

#[tauri::command]
pub fn get_active_processes(state: State<'_, AppState>) -> Vec<ProcessNetworkReport> {
    let mut monitor = state.process_monitor.lock().unwrap();
    monitor.snapshot_processes_with_detector(Some(&state.streaming_detector))
}

#[tauri::command]
pub fn get_optimization_state(state: State<'_, AppState>) -> OptimizationState {
    let diag = state.latency_diag.lock().unwrap();
    let mgr = state.bandwidth_mgr_arc.lock().unwrap();

    let active_downloads = state
        .download_engine
        .get_all_tasks()
        .iter()
        .filter(|t| t.status == crate::downloads::types::DownloadStatus::Downloading)
        .count() as u32;

    let active_streams = {
        let mut monitor = state.process_monitor.lock().unwrap();
        monitor
            .snapshot_processes_with_detector(Some(&state.streaming_detector))
            .iter()
            .filter(|p| p.is_streaming)
            .count() as u32
    };

    mgr.get_state(diag.last_latency_ms, diag.jitter_ms, active_streams, active_downloads)
}

#[tauri::command]
pub fn set_optimization_mode(state: State<'_, AppState>, mode: OperatingMode) -> OptimizationState {
    let diag = state.latency_diag.lock().unwrap();
    let mut mgr = state.bandwidth_mgr_arc.lock().unwrap();

    let active_downloads = state
        .download_engine
        .get_all_tasks()
        .iter()
        .filter(|t| t.status == crate::downloads::types::DownloadStatus::Downloading)
        .count() as u32;

    let active_streams = {
        let mut monitor = state.process_monitor.lock().unwrap();
        monitor
            .snapshot_processes_with_detector(Some(&state.streaming_detector))
            .iter()
            .filter(|p| p.is_streaming)
            .count() as u32
    };

    mgr.set_mode(mode);
    mgr.get_state(diag.last_latency_ms, diag.jitter_ms, active_streams, active_downloads)
}

#[tauri::command]
pub fn toggle_optimization(state: State<'_, AppState>, enabled: bool) -> OptimizationState {
    let diag = state.latency_diag.lock().unwrap();
    let mut mgr = state.bandwidth_mgr_arc.lock().unwrap();

    let active_downloads = state
        .download_engine
        .get_all_tasks()
        .iter()
        .filter(|t| t.status == crate::downloads::types::DownloadStatus::Downloading)
        .count() as u32;

    let active_streams = {
        let mut monitor = state.process_monitor.lock().unwrap();
        monitor
            .snapshot_processes_with_detector(Some(&state.streaming_detector))
            .iter()
            .filter(|p| p.is_streaming)
            .count() as u32
    };

    mgr.set_enabled(enabled);
    mgr.get_state(diag.last_latency_ms, diag.jitter_ms, active_streams, active_downloads)
}
