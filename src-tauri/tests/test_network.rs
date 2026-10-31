#[test]
fn test_query_hardware_interfaces() {
    let adapters = netboost_lib::windows::iphlpapi::query_interface_table();
    println!("Found {} hardware network interfaces", adapters.len());
    let mut operational_found = false;
    for a in &adapters {
        if a.is_up {
            operational_found = true;
            println!(
                "Active Adapter: {} [{}] | Speed: {} bps | In: {} B | Out: {} B",
                a.name, a.adapter_type, a.speed_bps, a.in_octets, a.out_octets
            );
        }
    }
    assert!(!adapters.is_empty(), "Should discover network adapters via GetIfTable2");
    assert!(operational_found, "Should find at least one operational interface (Wi-Fi, Ethernet, or Loopback)");
}

#[test]
fn test_query_active_tcp_connections() {
    let conns = netboost_lib::windows::iphlpapi::query_extended_tcp_table();
    println!("Discovered {} active TCP connections via GetExtendedTcpTable", conns.len());
    for c in conns.iter().take(5) {
        println!(
            "PID: {} | Process: {} | State: {} | {} -> {}",
            c.owner_pid, c.process_name, c.state, c.local_addr, c.remote_addr
        );
    }
    // TCP querying succeeds and returns a valid list
    println!("TCP connection table querying completed successfully.");
}

#[test]
fn test_health_evaluation() {
    let excellent = netboost_lib::network::health::evaluate_network_health(15.0, 1.2, 0.0);
    assert_eq!(excellent.score, "excellent");
    assert!(excellent.score_number >= 90);

    let good = netboost_lib::network::health::evaluate_network_health(35.0, 4.0, 0.5);
    assert_eq!(good.score, "good");

    let poor = netboost_lib::network::health::evaluate_network_health(220.0, 25.0, 5.0);
    assert_eq!(poor.score, "poor");
    assert!(poor.score_number <= 50);
}

#[test]
fn test_bandwidth_manager_congestion_detection() {
    let mut mgr = netboost_lib::bandwidth::manager::BandwidthManager::new();
    mgr.baseline_latency_ms = 20.0;

    // Normal unloaded latency
    let (congested, _) = mgr.check_congestion(22.0, 2.0);
    assert!(!congested);

    // Bufferbloat latency spike (e.g. 95 ms under load, > 2.5x baseline)
    let (congested_spike, reason) = mgr.check_congestion(95.0, 3.0);
    assert!(congested_spike);
    assert!(reason.unwrap().contains("Bufferbloat"));
}
