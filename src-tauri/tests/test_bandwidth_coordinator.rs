use netboost_lib::bandwidth::manager::{BandwidthManager, OperatingMode};

#[test]
fn test_single_spike_does_not_trigger_bufferbloat() {
    let mut mgr = BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // Normal baseline measurements
    for _ in 0..10 {
        mgr.record_probe(20.0, 1.5, 0.0);
    }

    assert_eq!(mgr.calculate_median_latency(), 20.0);

    // Single outlier spike (e.g. temporary Wi-Fi packet retransmit)
    let (is_congested, _) = mgr.evaluate_congestion_with_traffic(150.0, 25.0, 1, 5_000_000.0, 0);

    // Single spike MUST NOT trigger bufferbloat (hysteresis streak < 3)
    assert!(!is_congested, "Single latency spike should not trigger bufferbloat");
    assert!(!mgr.is_congested);
}

#[test]
fn test_hysteresis_requires_sustained_congestion_cycles() {
    let mut mgr = BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // Cycle 1: High latency
    let (c1, _) = mgr.evaluate_congestion_with_traffic(90.0, 20.0, 1, 4_000_000.0, 0);
    assert!(!c1, "Cycle 1 should not trigger congestion");

    // Cycle 2: High latency
    let (c2, _) = mgr.evaluate_congestion_with_traffic(95.0, 22.0, 1, 4_000_000.0, 0);
    assert!(!c2, "Cycle 2 should not trigger congestion");

    // Cycle 3: High latency -> Hysteresis threshold reached (streak >= 3)
    let (c3, reason) = mgr.evaluate_congestion_with_traffic(100.0, 24.0, 1, 4_000_000.0, 0);
    assert!(c3, "Cycle 3 must trigger congestion after 3 sustained cycles");
    assert!(mgr.is_congested);
    assert!(reason.is_some());
}

#[test]
fn test_causality_distinguishes_local_downloads_from_external_congestion() {
    let mut mgr = BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // Case A: High latency with NO NetBoost download activity (active_downloads = 0, speed = 0)
    for _ in 0..5 {
        mgr.evaluate_congestion_with_traffic(120.0, 25.0, 0, 0.0, 0);
    }
    assert!(mgr.is_congested);
    let state = mgr.get_state(120.0, 25.0, 0, 0);
    let reason = state.congestion_reason.unwrap_or_default();
    assert!(
        reason.contains("External network congestion") || reason.contains("NetBoost downloads idle"),
        "Should correctly attribute congestion as external when NetBoost is idle, got: {}",
        reason
    );

    // Case B: High latency WITH active NetBoost downloads
    let mut mgr2 = BandwidthManager::new();
    mgr2.baseline_latency_ms = 20.0;
    for _ in 0..5 {
        mgr2.evaluate_congestion_with_traffic(120.0, 25.0, 1, 3_500_000.0, 0);
    }
    assert!(mgr2.is_congested);
    let (_, r2) = mgr2.evaluate_congestion_with_traffic(120.0, 25.0, 1, 3_500_000.0, 0);
    let reason2 = r2.unwrap_or_default();
    assert!(
        reason2.contains("Bufferbloat under download load") || reason2.contains("pacing"),
        "Should attribute congestion to download load, got: {}",
        reason2
    );
}

#[test]
fn test_recovery_hysteresis_requires_multiple_calm_cycles() {
    let mut mgr = BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // Induce congestion
    for _ in 0..5 {
        mgr.evaluate_congestion_with_traffic(100.0, 20.0, 1, 2_000_000.0, 0);
    }
    assert!(mgr.is_congested);

    // 1 calm cycle is NOT enough to declare recovery
    mgr.evaluate_congestion_with_traffic(21.0, 2.0, 1, 2_000_000.0, 0);
    assert!(mgr.is_congested, "1 calm cycle must not clear congestion");

    // 4 calm cycles is still not enough
    for _ in 0..3 {
        mgr.evaluate_congestion_with_traffic(21.0, 2.0, 1, 2_000_000.0, 0);
    }
    assert!(mgr.is_congested, "4 calm cycles must not clear congestion");

    // 5th calm cycle triggers recovery
    mgr.evaluate_congestion_with_traffic(21.0, 2.0, 1, 2_000_000.0, 0);
    assert!(!mgr.is_congested, "5 consecutive calm cycles must declare recovery");
}

#[test]
fn test_operating_modes_pacing_and_concurrency() {
    let mut mgr = BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // 1. Streaming Priority Mode
    mgr.set_mode(OperatingMode::Streaming);
    for _ in 0..5 {
        mgr.evaluate_congestion_with_traffic(95.0, 22.0, 1, 2_000_000.0, 1);
    }
    assert!(mgr.get_pacing_delay_ms() >= 25, "Streaming mode should apply strong pacing >= 25ms");
    assert_eq!(mgr.get_effective_concurrency(4), 2, "Streaming mode should halve concurrency under load");

    // 2. Download Priority Mode
    mgr.set_mode(OperatingMode::Download);
    mgr.evaluate_congestion_with_traffic(95.0, 22.0, 1, 2_000_000.0, 0);
    assert_eq!(mgr.get_pacing_delay_ms(), 0, "Download mode should prioritize throughput (0ms pace)");
    assert_eq!(mgr.get_effective_concurrency(4), 4, "Download mode maintains full concurrency");
}

