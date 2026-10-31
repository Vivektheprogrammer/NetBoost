use crate::commands::network::AppState;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::Command;
use tauri::State;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(rename = "minimizeToTray")]
    pub minimize_to_tray: bool,
    #[serde(rename = "launchOnStartup")]
    pub launch_on_startup: bool,
    #[serde(rename = "bufferbloatWarning")]
    pub bufferbloat_warning: bool,
    #[serde(rename = "defaultMode")]
    pub default_mode: String,
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    let min_tray = state
        .db
        .get_setting("minimize_to_tray")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);

    let launch_startup = get_launch_on_startup_internal();

    let warning = state
        .db
        .get_setting("bufferbloat_warning")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);

    let mode = state
        .db
        .get_setting("default_mode")
        .ok()
        .flatten()
        .unwrap_or_else(|| "streaming".to_string());

    AppSettings {
        minimize_to_tray: min_tray,
        launch_on_startup: launch_startup,
        bufferbloat_warning: warning,
        default_mode: mode,
    }
}

#[tauri::command]
pub fn save_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let _ = state.db.set_setting("minimize_to_tray", if settings.minimize_to_tray { "true" } else { "false" });
    let _ = state.db.set_setting("bufferbloat_warning", if settings.bufferbloat_warning { "true" } else { "false" });
    let _ = state.db.set_setting("default_mode", &settings.default_mode);

    // Apply startup registry setting
    set_launch_on_startup_internal(settings.launch_on_startup)?;

    state.logger.log(
        "INFO",
        &format!(
            "Saved user settings: tray={}, startup={}, mode={}",
            settings.minimize_to_tray, settings.launch_on_startup, settings.default_mode
        ),
    );

    Ok(())
}

#[tauri::command]
pub fn get_application_logs(state: State<'_, AppState>) -> Vec<String> {
    state.logger.get_recent_logs(100)
}

fn get_launch_on_startup_internal() -> bool {
    let output = Command::new("reg")
        .args(["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "NetBoost"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

fn set_launch_on_startup_internal(enable: bool) -> Result<(), String> {
    if enable {
        if let Ok(exe) = std::env::current_exe() {
            let exe_str = exe.to_string_lossy().to_string();
            let _ = Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "NetBoost",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\"", exe_str),
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Registry write error: {}", e))?;
        }
    } else {
        let _ = Command::new("reg")
            .args([
                "delete",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "NetBoost",
                "/f",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    Ok(())
}

