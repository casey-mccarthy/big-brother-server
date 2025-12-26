use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use inventory_server::{db, handlers, AppState};
use tempfile::NamedTempFile;

/// Creates a test application with a temporary SQLite database.
/// Returns the router and the temp file (which must be kept alive for the duration of the test).
pub fn setup_test_app() -> (Router, NamedTempFile) {
    let temp_db = NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = temp_db.path().to_str().unwrap().to_string();

    // Initialize the database schema
    db::open_and_init(&db_path).expect("Failed to initialize test database");

    let state = Arc::new(AppState {
        db_path,
        debug_mode: false,
    });

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/device/:serial", get(handlers::device_detail))
        .route("/checkin", post(handlers::checkin))
        .with_state(state);

    (app, temp_db)
}

/// Creates a valid check-in JSON payload for testing.
pub fn valid_checkin_json() -> String {
    serde_json::json!({
        "hostname": "TEST-LAPTOP-001",
        "laptop_serial": "SN123456789",
        "ip_address": "192.168.1.100",
        "logged_in_user": "testuser",
        "timestamp_utc": "2024-01-15T10:30:00Z",
        "drives": [
            {
                "device_id": "\\\\.\\PhysicalDrive0",
                "model": "Samsung SSD 970 EVO",
                "serial_number": "S4EVNX0M123456",
                "size_bytes": 500107862016_i64,
                "media_type": "SSD"
            }
        ]
    })
    .to_string()
}

/// Creates a check-in JSON with custom values.
pub fn checkin_json_with(
    hostname: &str,
    serial: &str,
    ip: &str,
    user: Option<&str>,
    timestamp: &str,
) -> String {
    serde_json::json!({
        "hostname": hostname,
        "laptop_serial": serial,
        "ip_address": ip,
        "logged_in_user": user,
        "timestamp_utc": timestamp,
        "drives": []
    })
    .to_string()
}
