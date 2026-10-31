use netboost_lib::logging::NetBoostLogger;
use netboost_lib::storage::db::{Database, DbDownloadRecord};

#[test]
fn test_app_settings_persistence() {
    let db = Database::in_memory().unwrap();

    db.set_setting("minimize_to_tray", "true").unwrap();
    db.set_setting("launch_on_startup", "false").unwrap();
    db.set_setting("default_mode", "streaming").unwrap();

    assert_eq!(db.get_setting("minimize_to_tray").unwrap(), Some("true".to_string()));
    assert_eq!(db.get_setting("launch_on_startup").unwrap(), Some("false".to_string()));
    assert_eq!(db.get_setting("default_mode").unwrap(), Some("streaming".to_string()));
}

#[test]
fn test_crash_recovery_resets_interrupted_downloads() {
    let db = Database::in_memory().unwrap();

    let rec1 = DbDownloadRecord {
        id: "dl-1".to_string(),
        url: "http://example.com/file1.bin".to_string(),
        filename: "file1.bin".to_string(),
        destination_path: "C:\\Downloads\\file1.bin".to_string(),
        total_bytes: 10000,
        downloaded_bytes: 4000,
        status: "downloading".to_string(), // Active when crash occurs
        etag: None,
        last_modified: None,
        checksum_sha256: None,
        error_message: None,
        concurrency: 4,
        created_at: 1000,
        completed_at: None,
    };

    let rec2 = DbDownloadRecord {
        id: "dl-2".to_string(),
        url: "http://example.com/file2.bin".to_string(),
        filename: "file2.bin".to_string(),
        destination_path: "C:\\Downloads\\file2.bin".to_string(),
        total_bytes: 5000,
        downloaded_bytes: 5000,
        status: "completed".to_string(),
        etag: None,
        last_modified: None,
        checksum_sha256: None,
        error_message: None,
        concurrency: 2,
        created_at: 500,
        completed_at: Some(900),
    };

    db.save_download(&rec1).unwrap();
    db.save_download(&rec2).unwrap();

    // Trigger startup crash recovery
    let recovered_count = db.recover_interrupted_downloads().unwrap();
    assert_eq!(recovered_count, 1, "Should recover exactly 1 interrupted download");

    let all = db.get_all_downloads().unwrap();
    let d1 = all.iter().find(|d| d.id == "dl-1").unwrap();
    assert_eq!(d1.status, "paused", "Interrupted download must be safely reset to paused");

    let d2 = all.iter().find(|d| d.id == "dl-2").unwrap();
    assert_eq!(d2.status, "completed", "Completed download must remain untouched");
}

#[test]
fn test_logger_writes_and_retrieves_entries() {
    let logger = NetBoostLogger::init();
    logger.log("INFO", "Integration test log entry 1");
    logger.log("WARN", "Integration test log entry 2");

    let logs = logger.get_recent_logs(10);
    assert!(logs.iter().any(|l| l.contains("Integration test log entry 1")));
    assert!(logs.iter().any(|l| l.contains("Integration test log entry 2")));
}

