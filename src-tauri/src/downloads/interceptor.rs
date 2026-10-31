use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedDownload {
    pub id: Option<u64>,
    pub url: String,
    pub filename: Option<String>,
    pub referrer: Option<String>,
    pub cookies: Option<String>,
    pub source: String, // "browser_extension" or "clipboard"
}

// Global registry of pending browser download confirmation channels using standard OnceLock
static PENDING_DOWNLOADS: OnceLock<Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>> = OnceLock::new();

fn get_pending_map() -> Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>> {
    PENDING_DOWNLOADS.get_or_init(|| Arc::new(Mutex::new(HashMap::new()))).clone()
}

/// Called by frontend to resolve a pending download:
/// accept = true -> download with NetBoost (cancel in Chrome)
/// accept = false -> download normally in Chrome (resume in Chrome)
pub fn resolve_download_choice(id: u64, accept: bool) {
    let map = get_pending_map();
    let mut pending = map.lock().unwrap();
    if let Some(sender) = pending.remove(&id) {
        let _ = sender.send(accept);
    }
}

/// Start the background local IPC server for browser download button interception
pub fn start_download_interceptor(app_handle: AppHandle) {
    let app_clone = app_handle.clone();

    // Local HTTP IPC listener on 127.0.0.1:52140 for the Chrome/Edge extension
    tauri::async_runtime::spawn(async move {
        run_interceptor_server(app_clone).await;
    });
}

/// Runs a lightweight HTTP server on localhost:52140 for Chrome/Edge extensions
async fn run_interceptor_server(app: AppHandle) {
    let addr = "127.0.0.1:52140";
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            log::warn!("NetBoost download interceptor failed to bind {}: {}", addr, e);
            return;
        }
    };

    log::info!("NetBoost download interceptor listening on http://{}", addr);

    loop {
        if let Ok((mut socket, _)) = listener.accept().await {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let mut buffer = [0u8; 16384];
                let n = match socket.read(&mut buffer).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };

                let request = String::from_utf8_lossy(&buffer[..n]);

                // Handle CORS preflight
                if request.starts_with("OPTIONS ") {
                    let response = "HTTP/1.1 200 OK\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                        Access-Control-Allow-Headers: Content-Type\r\n\
                        Content-Length: 0\r\n\r\n";
                    let _ = socket.write_all(response.as_bytes()).await;
                    return;
                }

                // Handle ping/status
                if request.starts_with("GET /status") || request.starts_with("GET / ") {
                    let body = r#"{"status":"online","app":"NetBoost"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Content-Type: application/json\r\n\
                        Content-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    return;
                }

                // Handle POST /intercept
                if request.starts_with("POST /intercept") {
                    if let Some(body_start) = request.find("\r\n\r\n") {
                        let json_str = &request[body_start + 4..];
                        if let Ok(mut download) = serde_json::from_str::<InterceptedDownload>(json_str) {
                            download.source = "browser_extension".to_string();
                            let dl_id = download.id.unwrap_or(0);

                            // Setup oneshot channel for user's decision
                            let (tx, rx) = oneshot::channel::<bool>();
                            if dl_id > 0 {
                                get_pending_map().lock().unwrap().insert(dl_id, tx);
                            }

                            // Emit event to Tauri frontend
                            let _ = app_handle.emit("netboost-download-detected", &download);

                            // Bring NetBoost window to the foreground
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }

                            // Wait for user's decision or timeout after 25 seconds
                            let user_accepted = if dl_id > 0 {
                                match tokio::time::timeout(Duration::from_secs(25), rx).await {
                                    Ok(Ok(val)) => val,
                                    _ => {
                                        // Timeout or cancelled: do not force acceleration; let browser download normally
                                        get_pending_map().lock().unwrap().remove(&dl_id);
                                        false
                                    }
                                }
                            } else {
                                true
                            };

                            let body = format!(r#"{{"accelerate":{}}}"#, user_accepted);
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                Access-Control-Allow-Origin: *\r\n\
                                Content-Type: application/json\r\n\
                                Content-Length: {}\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                            return;
                        }
                    }
                }

                // Default 404 response
                let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
            });
        }
    }
}
