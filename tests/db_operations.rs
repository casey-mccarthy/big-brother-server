use inventory_server::db;
use rusqlite::params;
use tempfile::NamedTempFile;

#[test]
fn test_open_and_init_creates_tables() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).expect("Failed to initialize database");

    // Verify laptops table exists
    let laptops_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='laptops'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(laptops_exists, 1, "laptops table should exist");

    // Verify checkins table exists
    let checkins_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='checkins'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(checkins_exists, 1, "checkins table should exist");

    // Verify WAL mode is enabled
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_lowercase(), "wal", "WAL mode should be enabled");
}

#[test]
fn test_open_and_init_creates_indexes() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).expect("Failed to initialize database");

    // Verify indexes exist
    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_checkins_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 2, "should have 2 checkins indexes");
}

#[test]
fn test_get_all_laptops_empty_database() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();
    let laptops = db::get_all_laptops(&conn).expect("Failed to query laptops");

    assert!(laptops.is_empty(), "Empty database should return no laptops");
}

#[test]
fn test_get_all_laptops_returns_data() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();

    // Insert test data
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "192.168.1.1", "user1", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();

    let laptops = db::get_all_laptops(&conn).unwrap();

    assert_eq!(laptops.len(), 1);
    assert_eq!(laptops[0].laptop_serial, "SN001");
    assert_eq!(laptops[0].hostname, "laptop1");
}

#[test]
fn test_get_all_laptops_ordered_by_last_seen_desc() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();

    // Insert in non-chronological order
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "192.168.1.1", "user1", "2024-01-10T10:00:00Z", "[]"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN002", "laptop2", "192.168.1.2", "user2", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();

    let laptops = db::get_all_laptops(&conn).unwrap();

    assert_eq!(laptops.len(), 2);
    // Most recent should be first
    assert_eq!(laptops[0].laptop_serial, "SN002");
    assert_eq!(laptops[1].laptop_serial, "SN001");
}

#[test]
fn test_get_laptop_by_serial_not_found() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();
    let laptop = db::get_laptop_by_serial(&conn, "NONEXISTENT").unwrap();

    assert!(laptop.is_none(), "Should return None for unknown serial");
}

#[test]
fn test_get_laptop_by_serial_found() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();

    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN123", "test-laptop", "10.0.0.1", "admin", "2024-01-15T12:00:00Z", "[]"],
    )
    .unwrap();

    let laptop = db::get_laptop_by_serial(&conn, "SN123").unwrap();

    assert!(laptop.is_some());
    let laptop = laptop.unwrap();
    assert_eq!(laptop.laptop_serial, "SN123");
    assert_eq!(laptop.hostname, "test-laptop");
    assert_eq!(laptop.ip_address, "10.0.0.1");
}

#[test]
fn test_get_checkins_by_serial_empty() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();
    let checkins = db::get_checkins_by_serial(&conn, "UNKNOWN").unwrap();

    assert!(checkins.is_empty(), "Should return empty vec for unknown serial");
}

#[test]
fn test_get_checkins_by_serial_returns_history() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();

    // Insert multiple checkins for same laptop
    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "192.168.1.1", "user1", "2024-01-10T10:00:00Z", "[]"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "192.168.1.2", "user1", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();

    let checkins = db::get_checkins_by_serial(&conn, "SN001").unwrap();

    assert_eq!(checkins.len(), 2);
    // Most recent should be first (descending order)
    assert_eq!(checkins[0].timestamp_utc, "2024-01-15T10:00:00Z");
    assert_eq!(checkins[1].timestamp_utc, "2024-01-10T10:00:00Z");
}

#[test]
fn test_get_checkins_filters_by_serial() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = db::open_and_init(db_path).unwrap();

    // Insert checkins for different laptops
    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "192.168.1.1", "user1", "2024-01-10T10:00:00Z", "[]"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN002", "laptop2", "192.168.1.2", "user2", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();

    let checkins = db::get_checkins_by_serial(&conn, "SN001").unwrap();

    assert_eq!(checkins.len(), 1);
    assert_eq!(checkins[0].hostname, "laptop1");
}
