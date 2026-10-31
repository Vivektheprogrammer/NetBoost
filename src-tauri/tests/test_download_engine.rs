use netboost_lib::bandwidth::manager::{BandwidthManager, OperatingMode};
use netboost_lib::downloads::engine::DownloadEngine;
use netboost_lib::downloads::types::DownloadStatus;
use netboost_lib::storage::db::Database;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Helper to generate known deterministic byte buffer
fn generate_test_payload(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 251) as u8).collect()
}

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[tokio::test]
async fn test_download_server_with_http_range() {
    let payload = generate_test_payload(120_000); // 120 KB test payload
    let expected_sha = compute_sha256(&payload);
    let payload_arc = Arc::new(payload);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let payload_ref = payload_arc.clone();

    // Spawn mock HTTP server handling byte ranges
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let data = payload_ref.clone();
                tokio::spawn(async move {
                    let mut req_buf = [0u8; 2048];
                    let n = socket.read(&mut req_buf).await.unwrap_or(0);
                    let req_str = String::from_utf8_lossy(&req_buf[..n]);

                    let total_len = data.len();

                    if req_str.contains("Range: bytes=0-0") {
                        let header = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nContent-Range: bytes 0-0/{}\r\nContent-Length: 1\r\nETag: \"etag-test-123\"\r\n\r\n",
                            total_len
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&data[0..1]).await;
                    } else if let Some(pos) = req_str.find("Range: bytes=") {
                        let range_part = &req_str[pos + 13..];
                        if let Some(end_line) = range_part.find("\r\n") {
                            let range_val = &range_part[..end_line];
                            let parts: Vec<&str> = range_val.split('-').collect();
                            let start: usize = parts[0].parse().unwrap_or(0);
                            let end: usize = parts[1].parse().unwrap_or(total_len - 1);
                            let chunk = &data[start..=end];

                            let header = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nETag: \"etag-test-123\"\r\n\r\n",
                                start, end, total_len, chunk.len()
                            );
                            let _ = socket.write_all(header.as_bytes()).await;
                            let _ = socket.write_all(chunk).await;
                        }
                    } else {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nETag: \"etag-test-123\"\r\n\r\n",
                            total_len
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&data).await;
                    }
                });
            }
        }
    });

    let temp_dir = std::env::temp_dir().join(format!("nb-range-test-{}", port));
    let _ = std::fs::create_dir_all(&temp_dir);

    let db = Arc::new(Database::in_memory().unwrap());
    let engine = DownloadEngine::new(db, Some(temp_dir.clone()));
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));

    let url = format!("http://127.0.0.1:{}/range_file.bin", port);
    let id = engine
        .start_download(&url, Some("range_file.bin".to_string()), Some(4), b_mgr)
        .await
        .unwrap();

    // Poll until completed
    let mut completed = false;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let tasks = engine.get_all_tasks();
        if let Some(t) = tasks.iter().find(|t| t.id == id) {
            if t.status == DownloadStatus::Completed {
                completed = true;
                break;
            }
        }
    }

    assert!(completed, "Range-supported download should complete successfully");

    // Verify file content matches expected SHA-256
    let downloaded_file = temp_dir.join("range_file.bin");
    assert!(downloaded_file.exists(), "Target file must exist after completion");
    let content = std::fs::read(&downloaded_file).unwrap();
    assert_eq!(content.len(), 120_000);
    assert_eq!(compute_sha256(&content), expected_sha, "File integrity verified");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_download_server_without_range_support() {
    let payload = generate_test_payload(40_000);
    let expected_sha = compute_sha256(&payload);
    let payload_arc = Arc::new(payload);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let payload_ref = payload_arc.clone();

    // Mock server returning 200 OK without Accept-Ranges
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let data = payload_ref.clone();
                tokio::spawn(async move {
                    let mut req_buf = [0u8; 1024];
                    let _ = socket.read(&mut req_buf).await;

                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                        data.len()
                    );
                    let _ = socket.write_all(header.as_bytes()).await;
                    let _ = socket.write_all(&data).await;
                });
            }
        }
    });

    let temp_dir = std::env::temp_dir().join(format!("nb-norange-test-{}", port));
    let _ = std::fs::create_dir_all(&temp_dir);

    let db = Arc::new(Database::in_memory().unwrap());
    let engine = DownloadEngine::new(db, Some(temp_dir.clone()));
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));

    let url = format!("http://127.0.0.1:{}/norange_file.bin", port);
    let id = engine
        .start_download(&url, Some("norange_file.bin".to_string()), Some(4), b_mgr)
        .await
        .unwrap();

    let mut completed = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let tasks = engine.get_all_tasks();
        if let Some(t) = tasks.iter().find(|t| t.id == id) {
            if t.status == DownloadStatus::Completed {
                completed = true;
                break;
            }
        }
    }

    assert!(completed, "Non-range server download should fallback to single worker and complete");

    let downloaded_file = temp_dir.join("norange_file.bin");
    let content = std::fs::read(&downloaded_file).unwrap();
    assert_eq!(compute_sha256(&content), expected_sha);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_download_server_http_429_retry_after() {
    let payload = generate_test_payload(15_000);
    let payload_arc = Arc::new(payload);
    let request_count = Arc::new(AtomicU32::new(0));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let req_counter = request_count.clone();

    // Mock server returning 429 Too Many Requests on first probe, then succeeds
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let data = payload_arc.clone();
                let cnt = req_counter.clone();
                tokio::spawn(async move {
                    let mut req_buf = [0u8; 1024];
                    let _ = socket.read(&mut req_buf).await;

                    let attempt = cnt.fetch_add(1, Ordering::SeqCst);
                    if attempt == 0 {
                        // Return HTTP 429 Too Many Requests with Retry-After: 1
                        let header = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nContent-Length: 0\r\n\r\n";
                        let _ = socket.write_all(header.as_bytes()).await;
                    } else {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                            data.len()
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&data).await;
                    }
                });
            }
        }
    });

    let temp_dir = std::env::temp_dir().join(format!("nb-429-test-{}", port));
    let _ = std::fs::create_dir_all(&temp_dir);

    let db = Arc::new(Database::in_memory().unwrap());
    let engine = DownloadEngine::new(db, Some(temp_dir.clone()));
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));

    let url = format!("http://127.0.0.1:{}/rate_limited.bin", port);
    let res = engine
        .start_download(&url, Some("rate_limited.bin".to_string()), Some(1), b_mgr)
        .await;

    assert!(res.is_ok(), "Engine should handle HTTP 429 and succeed on retry");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_pause_and_resume_download() {
    let payload = generate_test_payload(80_000);
    let expected_sha = compute_sha256(&payload);
    let payload_arc = Arc::new(payload.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let payload_ref = payload_arc.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let data = payload_ref.clone();
                tokio::spawn(async move {
                    let mut req_buf = [0u8; 1024];
                    let n = socket.read(&mut req_buf).await.unwrap_or(0);
                    let req_str = String::from_utf8_lossy(&req_buf[..n]);
                    let req_lower = req_str.to_lowercase();

                    let total_len = data.len();
                    if req_lower.contains("range: bytes=0-0") {
                        let header = format!(
                            "HTTP/1.1 206 Partial Content\r\nAccept-Ranges: bytes\r\nContent-Range: bytes 0-0/{}\r\nContent-Length: 1\r\nETag: \"etag-resume-1\"\r\n\r\n",
                            total_len
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&data[0..1]).await;
                    } else if let Some(pos) = req_lower.find("range: bytes=") {
                        let range_part = &req_lower[pos + 13..];
                        if let Some(end_line) = range_part.find("\r\n") {
                            let range_val = &range_part[..end_line];
                            let parts: Vec<&str> = range_val.split('-').collect();
                            let start: usize = parts[0].trim().parse().unwrap_or(0);
                            let end: usize = parts[1].trim().parse().unwrap_or(total_len - 1);
                            let chunk = &data[start..=end];

                            let header = format!(
                                "HTTP/1.1 206 Partial Content\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nETag: \"etag-resume-1\"\r\n\r\n",
                                start, end, total_len, chunk.len()
                            );
                            let _ = socket.write_all(header.as_bytes()).await;

                            if chunk.len() > 100 {
                                let mid = chunk.len() / 2;
                                let _ = socket.write_all(&chunk[..mid]).await;
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                let _ = socket.write_all(&chunk[mid..]).await;
                            } else {
                                let _ = socket.write_all(chunk).await;
                            }
                        }
                    } else {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nETag: \"etag-resume-1\"\r\n\r\n",
                            total_len
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&data).await;
                    }
                });
            }
        }
    });

    let temp_dir = std::env::temp_dir().join(format!("nb-resume-test-{}", port));
    let _ = std::fs::create_dir_all(&temp_dir);

    let db = Arc::new(Database::in_memory().unwrap());
    let engine = DownloadEngine::new(db, Some(temp_dir.clone()));
    let b_mgr = Arc::new(Mutex::new(BandwidthManager::new()));

    let url = format!("http://127.0.0.1:{}/resumable.bin", port);
    let id = engine
        .start_download(&url, Some("resumable.bin".to_string()), Some(2), b_mgr.clone())
        .await
        .unwrap();

    // Pause download mid-transfer
    tokio::time::sleep(Duration::from_millis(15)).await;
    engine.pause_download(&id).unwrap();

    let tasks = engine.get_all_tasks();
    let task = tasks.iter().find(|t| t.id == id).unwrap();
    assert_eq!(task.status, DownloadStatus::Paused);

    // Resume download
    let resume_res = engine.resume_download(&id, b_mgr).await;
    assert!(resume_res.is_ok(), "Resume should succeed with matching ETag");

    // Wait for completion
    let mut completed = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let tasks = engine.get_all_tasks();
        if let Some(t) = tasks.iter().find(|t| t.id == id) {
            if t.status == DownloadStatus::Completed {
                completed = true;
                break;
            }
        }
    }

    assert!(completed, "Resumed download should complete fully");

    let downloaded_file = temp_dir.join("resumable.bin");
    let content = std::fs::read(&downloaded_file).unwrap();
    println!("Downloaded length: {}, expected: {}", content.len(), payload.len());
    let mut mismatch_count = 0;
    for (i, (&b1, &b2)) in content.iter().zip(payload.iter()).enumerate() {
        if b1 != b2 && mismatch_count < 10 {
            println!("Mismatch at byte {}: downloaded={}, expected={}", i, b1, b2);
            mismatch_count += 1;
        }
    }
    assert_eq!(compute_sha256(&content), expected_sha);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_bandwidth_coordination_pacing() {
    let mut mgr = BandwidthManager::new();

    // Mode 1: Streaming Priority with high latency -> bufferbloat detected
    mgr.mode = OperatingMode::Streaming;
    mgr.enabled = true;
    mgr.baseline_latency_ms = 18.0;

    let (congested, reason) = mgr.check_congestion(80.0, 4.0);
    assert!(congested, "Bufferbloat must be flagged in Streaming mode when latency spikes");
    assert!(reason.unwrap().contains("Bufferbloat"));

    // Mode 2: Balanced Mode
    mgr.mode = OperatingMode::Balanced;
    let state = mgr.get_state(20.0, 2.0, 1, 2);
    assert_eq!(state.mode, OperatingMode::Balanced);
    assert!(state.policy_description.contains("Balanced"));

    // Mode 3: Download Priority
    mgr.mode = OperatingMode::Download;
    let state_dl = mgr.get_state(20.0, 2.0, 0, 4);
    assert_eq!(state_dl.mode, OperatingMode::Download);
    assert!(state_dl.policy_description.contains("Download"));
}
