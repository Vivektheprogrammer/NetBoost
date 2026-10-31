use crate::commands::network::AppState;
use crate::downloads::types::DownloadTask;
use std::process::Command;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    url: String,
    custom_filename: Option<String>,
    concurrency: Option<usize>,
) -> Result<String, String> {
    let engine = state.download_engine.clone();
    let b_mgr = state.bandwidth_mgr_arc.clone();

    engine
        .start_download(&url, custom_filename, concurrency, b_mgr)
        .await
}

#[tauri::command]
pub fn pause_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.download_engine.pause_download(&id)
}

#[tauri::command]
pub async fn resume_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let engine = state.download_engine.clone();
    let b_mgr = state.bandwidth_mgr_arc.clone();
    engine.resume_download(&id, b_mgr).await
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.download_engine.cancel_download(&id)
}

#[tauri::command]
pub async fn retry_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let engine = state.download_engine.clone();
    let b_mgr = state.bandwidth_mgr_arc.clone();
    engine.retry_download(&id, b_mgr).await
}

#[tauri::command]
pub fn get_downloads(state: State<'_, AppState>) -> Vec<DownloadTask> {
    state.download_engine.get_all_tasks()
}

#[tauri::command]
pub fn open_downloaded_file(path: String) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(format!("/select,\"{}\"", path))
        .spawn()
        .map_err(|e| format!("Failed to open file in Explorer: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn open_download_folder(path: String) -> Result<(), String> {
    let dir = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(path);

    Command::new("explorer.exe")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("Failed to open directory: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn resolve_intercepted_download(app: AppHandle, id: u64, accept: bool) {
    crate::downloads::interceptor::resolve_download_choice(id, accept);
    if !accept {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid or unsupported URL scheme".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&url)
            .spawn();

        if status.is_err() {
            Command::new("explorer.exe")
                .arg(&url)
                .spawn()
                .map_err(|e| format!("Failed to open URL in browser: {}", e))?;
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_extension_folder(app: AppHandle) -> Result<(), String> {
    let mut found_dir = None;

    // 1. Check bundled resource dir (in installed application)
    if let Ok(res_dir) = app.path().resource_dir() {
        let candidate = res_dir.join("extension");
        if candidate.exists() {
            found_dir = Some(candidate);
        } else {
            let candidate_up = res_dir.join("_up_").join("extension");
            if candidate_up.exists() {
                found_dir = Some(candidate_up);
            }
        }
    }

    // 2. Relative to current running executable
    if found_dir.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let candidate = exe_dir.join("resources").join("extension");
                if candidate.exists() {
                    found_dir = Some(candidate);
                } else {
                    let candidate2 = exe_dir.join("extension");
                    if candidate2.exists() {
                        found_dir = Some(candidate2);
                    }
                }
            }
        }
    }

    // 3. Fallback to development workspace
    if found_dir.is_none() {
        let dev1 = std::path::PathBuf::from("extension");
        if dev1.exists() {
            found_dir = Some(dev1);
        } else {
            let dev2 = std::path::PathBuf::from("../extension");
            if dev2.exists() {
                found_dir = Some(dev2);
            }
        }
    }

    if let Some(dir) = found_dir {
        Command::new("explorer.exe")
            .arg(dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open extension directory: {}", e))?;
        Ok(())
    } else {
        Err("Extension folder not found. Please verify application installation.".to_string())
    }
}
