use netboost_lib::streaming::detector::StreamingDetector;

#[test]
fn test_user_configured_streaming_application_override() {
    let detector = StreamingDetector::new();

    // Default: 'customstreamer.exe' is unknown
    let (is_stream, conf, _) = detector.classify_process("customstreamer.exe", 2, &["104.16.1.1:443".to_string()], 100_000.0);
    assert!(!is_stream);
    assert_eq!(conf, "unknown");

    // Add user configured app
    detector.add_user_app("customstreamer.exe");

    // Now it should be classified as High confidence streaming
    let (is_stream_after, conf_after, reason) = detector.classify_process("customstreamer.exe", 2, &["104.16.1.1:443".to_string()], 100_000.0);
    assert!(is_stream_after);
    assert_eq!(conf_after, "high");
    assert!(reason.unwrap().contains("User-configured"));
}

#[test]
fn test_dedicated_media_players() {
    let detector = StreamingDetector::new();

    let (is_netflix, conf_netflix, _) = detector.classify_process("netflix.exe", 4, &["198.51.100.1:443".to_string()], 3_000_000.0);
    assert!(is_netflix);
    assert_eq!(conf_netflix, "high");

    let (is_spotify, conf_spotify, _) = detector.classify_process("spotify.exe", 3, &["104.199.65.1:443".to_string()], 320_000.0);
    assert!(is_spotify);
    assert_eq!(conf_spotify, "high");
}

#[test]
fn test_browser_dash_hls_cadence_classification() {
    let detector = StreamingDetector::new();

    let endpoints = vec![
        "142.250.190.46:443".to_string(),
        "142.250.190.47:443".to_string(),
        "142.250.190.48:443".to_string(),
    ];

    // High sustained bitrate (> 1.5 Mbps) over multiple HTTPS connections
    let (is_stream, conf, reason) = detector.classify_process("chrome.exe", 3, &endpoints, 2_800_000.0);
    assert!(is_stream);
    assert_eq!(conf, "high");
    assert!(reason.unwrap().contains("DASH/HLS profile"));

    // Moderate/browsing traffic (300 Kbps) over HTTPS
    let (is_stream_med, conf_med, _) = detector.classify_process("chrome.exe", 2, &endpoints[0..2], 300_000.0);
    assert!(is_stream_med);
    assert_eq!(conf_med, "medium");

    // Idle or single background socket
    let (is_stream_low, conf_low, _) = detector.classify_process("chrome.exe", 1, &endpoints[0..1], 10_000.0);
    assert!(!is_stream_low);
    assert_eq!(conf_low, "low");
}

#[test]
fn test_cdn_domain_endpoint_matching() {
    let detector = StreamingDetector::new();

    let endpoints = vec![
        "rr1---sn-googlevideo.142.250.1.1:443".to_string(),
        "edge-nflxvideo.198.51.100.1:443".to_string(),
    ];

    let (is_stream, conf, reason) = detector.classify_process("msedge.exe", 2, &endpoints, 500_000.0);
    assert!(is_stream);
    assert_eq!(conf, "high");
    assert!(reason.unwrap().contains("streaming media CDN infrastructure"));
}

