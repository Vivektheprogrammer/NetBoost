pub mod bandwidth;
pub mod benchmark;
pub mod commands;
pub mod dns;
pub mod downloads;
pub mod logging;
pub mod network;
pub mod storage;
pub mod streaming;
pub mod windows;

use commands::network::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Start the background download interceptor (HTTP server on 127.0.0.1:52140)
            downloads::interceptor::start_download_interceptor(app.handle().clone());

            // Build system tray icon with menu
            let quit_item = MenuItem::with_id(app, "quit", "Quit NetBoost", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Open NetBoost", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .menu(&menu)
                    .tooltip("NetBoost Network Optimization Utility")
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "quit" => {
                                app.exit(0);
                            }
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent application termination on close; minimize to system tray instead
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Network & Diagnostics
            commands::network::get_realtime_metrics,
            commands::network::get_network_adapters,
            commands::network::get_active_processes,
            commands::network::get_optimization_state,
            commands::network::set_optimization_mode,
            commands::network::toggle_optimization,
            // Downloads Engine
            commands::downloads::start_download,
            commands::downloads::pause_download,
            commands::downloads::resume_download,
            commands::downloads::cancel_download,
            commands::downloads::retry_download,
            commands::downloads::get_downloads,
            commands::downloads::open_downloaded_file,
            commands::downloads::open_download_folder,
            commands::downloads::open_external_url,
            commands::downloads::open_extension_folder,
            commands::downloads::resolve_intercepted_download,
            commands::downloads::hide_window,
            // Benchmarking Engine
            commands::benchmark::run_benchmark,
            commands::benchmark::get_benchmark_history,
            commands::benchmark::clear_benchmark_history,
            commands::benchmark::get_benchmark_comparison,
            // Streaming Heuristics & Configuration
            commands::streaming::get_streaming_config,
            commands::streaming::add_user_streaming_app,
            commands::streaming::remove_user_streaming_app,
            // DNS Optimization & Restoration
            commands::dns::benchmark_dns,
            commands::dns::apply_dns_provider,
            commands::dns::restore_original_dns,
            commands::dns::get_dns_backup_status,
            // Settings, Tray & Production Controls
            commands::settings::get_app_settings,
            commands::settings::save_app_settings,
            commands::settings::get_application_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NetBoost application");
}
