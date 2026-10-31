use netboost_lib::dns::bench::DnsBenchmarkEngine;
use netboost_lib::storage::db::{Database, DnsBackupRecord};

#[test]
fn test_dns_packet_wire_format() {
    let query = DnsBenchmarkEngine::build_dns_query("google.com", 0x1234);

    // 12-byte header
    assert_eq!(&query[0..2], &[0x12, 0x34]); // Transaction ID
    assert_eq!(&query[2..4], &[0x01, 0x00]); // Standard query, recursion desired
    assert_eq!(&query[4..6], &[0x00, 0x01]); // 1 question

    // Question section: 6 'g' 'o' 'o' 'g' 'l' 'e' 3 'c' 'o' 'm' 0
    assert_eq!(query[12], 6);
    assert_eq!(&query[13..19], b"google");
    assert_eq!(query[19], 3);
    assert_eq!(&query[20..23], b"com");
    assert_eq!(query[23], 0); // null terminator

    // QTYPE: 0x0001 (A), QCLASS: 0x0001 (IN)
    assert_eq!(&query[24..26], &[0x00, 0x01]);
    assert_eq!(&query[26..28], &[0x00, 0x01]);
}

#[test]
fn test_dns_backup_and_restoration_persistence() {
    let db = Database::in_memory().unwrap();

    let backup = DnsBackupRecord {
        interface_name: "Wi-Fi".to_string(),
        is_dhcp: true,
        primary_dns: None,
        secondary_dns: None,
        backed_up_at: 1700000000,
    };

    db.save_dns_backup(&backup).unwrap();

    let retrieved = db.get_dns_backup("Wi-Fi").unwrap();
    assert!(retrieved.is_some());
    let r = retrieved.unwrap();
    assert_eq!(r.interface_name, "Wi-Fi");
    assert!(r.is_dhcp);

    // Test deletion on restoration
    db.delete_dns_backup("Wi-Fi").unwrap();
    let empty = db.get_dns_backup("Wi-Fi").unwrap();
    assert!(empty.is_none());
}

