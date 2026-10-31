use crate::commands::network::AppState;
use crate::dns::bench::{DnsBenchmarkEngine, DnsBenchmarkReport};
use crate::dns::windows::WindowsDnsManager;
use crate::storage::db::DnsBackupRecord;
use tauri::State;

#[tauri::command]
pub async fn benchmark_dns(state: State<'_, AppState>) -> Result<DnsBenchmarkReport, String> {
    // Find active adapter name
    let active_iface = {
        let mut tracker = state.adapter_tracker.lock().unwrap();
        let (_, active_opt, _, _) = tracker.poll_adapters();
        active_opt.map(|a| a.name)
    };

    let iface_name = active_iface.clone().unwrap_or_else(|| "Wi-Fi".to_string());

    // Query current DNS
    let (_is_dhcp, primary_opt, _) = WindowsDnsManager::get_interface_dns_config(&iface_name);

    tokio::task::spawn_blocking(move || {
        Ok(DnsBenchmarkEngine::benchmark_all(
            primary_opt.as_deref(),
            Some(&iface_name),
        ))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub fn apply_dns_provider(
    state: State<'_, AppState>,
    interface_name: String,
    primary_ip: String,
    secondary_ip: Option<String>,
) -> Result<(), String> {
    WindowsDnsManager::apply_dns(
        &state.db,
        &interface_name,
        &primary_ip,
        secondary_ip.as_deref(),
    )
}

#[tauri::command]
pub fn restore_original_dns(
    state: State<'_, AppState>,
    interface_name: String,
) -> Result<(), String> {
    WindowsDnsManager::restore_original_dns(&state.db, &interface_name)
}

#[tauri::command]
pub fn get_dns_backup_status(
    state: State<'_, AppState>,
    interface_name: String,
) -> Result<Option<DnsBackupRecord>, String> {
    state
        .db
        .get_dns_backup(&interface_name)
        .map_err(|e| format!("Database error: {}", e))
}

