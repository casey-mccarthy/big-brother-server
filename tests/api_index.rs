mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use rusqlite::params;
use tower::ServiceExt;

#[tokio::test]
async fn test_index_empty_database_returns_200() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_index_returns_html() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");

    assert!(
        content_type.contains("text/html"),
        "Expected HTML content type, got: {}",
        content_type
    );
}

#[tokio::test]
async fn test_index_shows_laptops() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    // Insert test laptop
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-TEST-123", "test-laptop", "192.168.1.100", "testuser", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    assert!(
        body_str.contains("SN-TEST-123"),
        "Page should contain laptop serial"
    );
    assert!(
        body_str.contains("test-laptop"),
        "Page should contain hostname"
    );
}

#[tokio::test]
async fn test_index_shows_drive_info() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    // Insert laptop with drive info
    let drives_json = serde_json::json!([{
        "device_id": "\\\\.\\PhysicalDrive0",
        "model": "Samsung SSD",
        "serial_number": "DRIVE-SERIAL-123"
    }])
    .to_string();

    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "10.0.0.1", "user1", "2024-01-15T10:00:00Z", drives_json],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    assert!(
        body_str.contains("DRIVE-SERIAL-123"),
        "Page should display drive serial number"
    );
}

#[tokio::test]
async fn test_index_escapes_agent_supplied_values() {
    // Drive serials and laptop serials come straight from agents. Server-side
    // validation allows any printable ASCII, so the template must escape them.
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let drives_json = serde_json::json!([
        {"device_id": "\\\\.\\PhysicalDrive0", "model": "Evil", "serial_number": "<script>alert(1)</script>"},
        {"device_id": "\\\\.\\PhysicalDrive1", "model": "Fine", "serial_number": "GOOD-SERIAL"}
    ])
    .to_string();

    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["X';alert(2);'", "host", "10.0.0.1", "user1", "2024-01-15T10:00:00Z", drives_json],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    assert!(
        !body_str.contains("<script>alert(1)</script>"),
        "drive serial must be escaped"
    );
    assert!(body_str.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(body_str.contains("GOOD-SERIAL<br>") || body_str.contains("<br>GOOD-SERIAL"));
    assert!(
        !body_str.contains("onclick"),
        "row links must not be inline JS built from the laptop serial"
    );
    assert!(body_str.contains("data-href=\"/device/X%27%3Balert%282%29%3B%27\""));
}
