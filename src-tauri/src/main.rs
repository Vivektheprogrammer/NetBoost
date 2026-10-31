// Prevents additional console window on Windows, DO NOT REMOVE!!
#![windows_subsystem = "windows"]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("CRASH PANIC: {}", info);
        let _ = std::fs::write("C:\\NetBoost\\panic.log", &msg);
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            let _ = std::fs::write(format!("{}\\NetBoost\\logs\\crash.log", appdata), &msg);
        }
    }));

    netboost_lib::run();
}
