use netboost_lib::bandwidth::manager::BandwidthManager;
use netboost_lib::benchmark::engine::{BenchmarkRequest, BenchmarkRunner};
use netboost_lib::storage::db::{Database, DbBenchmarkRecord};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_benchmark_runner_idle_and_history() {
    let db = Arc::new(Database::in_memory().unwrap());
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));
    let runner = BenchmarkRunner::new(db.clone(), b_mgr.clone());

    // Run idle baseline
    let req = BenchmarkRequest {
        scenario: "idle_only".to_string(),
        target_url: None,
    };

    let run1 = runner.run_benchmark(req).await.unwrap();
    assert_eq!(run1.scenario, "Idle Baseline");
    assert!(run1.unloaded_latency_ms > 0.0);

    // Verify stored in SQLite
    let history = db.get_benchmark_history().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, run1.id);
}

#[tokio::test]
async fn test_benchmark_comparison_calculation() {
    let db = Arc::new(Database::in_memory().unwrap());
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));
    let runner = BenchmarkRunner::new(db.clone(), b_mgr);

    // Save baseline run (unoptimized, high bufferbloat: 120ms)
    let baseline = DbBenchmarkRecord {
        id: "bm-baseline".to_string(),
        timestamp: 1000,
        mode: "download".to_string(),
        unloaded_latency_ms: 18.0,
        loaded_latency_ms: 120.0,
        jitter_ms: 15.0,
        packet_loss_percent: 0.0,
        download_speed_bps: 50_000_000.0,
        completion_time_secs: 5.0,
        optimization_enabled: false,
        scenario: "Unoptimized Baseline".to_string(),
        notes: None,
    };
    db.save_benchmark_run(&baseline).unwrap();

    // Save current run (streaming priority, suppressed bufferbloat: 32ms)
    let current = DbBenchmarkRecord {
        id: "bm-current".to_string(),
        timestamp: 2000,
        mode: "streaming".to_string(),
        unloaded_latency_ms: 18.0,
        loaded_latency_ms: 32.0,
        jitter_ms: 2.5,
        packet_loss_percent: 0.0,
        download_speed_bps: 44_000_000.0,
        completion_time_secs: 5.5,
        optimization_enabled: true,
        scenario: "Streaming Priority Run".to_string(),
        notes: None,
    };
    db.save_benchmark_run(&current).unwrap();

    let comp = runner.get_comparison("bm-current").unwrap();
    assert!(comp.baseline_run.is_some());
    // Delta should be 32 - 120 = -88 ms
    assert!((comp.latency_delta_ms - (-88.0)).abs() < 0.1);
    // Improvement percentage should be ((120 - 32) / 120) * 100 = 73.3%
    assert!(comp.latency_improvement_percent > 70.0);
    assert!(comp.summary.contains("Bufferbloat reduced"));
}
