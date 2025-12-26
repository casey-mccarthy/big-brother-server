mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use rusqlite::params;
use tower::ServiceExt;

#[tokio::test]
async fn test_device_not_found_returns_404() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/NONEXISTENT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_device_found_returns_200() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    // Insert test laptop
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-FOUND-001", "found-laptop", "192.168.1.50", "admin", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/SN-FOUND-001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_device_returns_html() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN001", "laptop1", "10.0.0.1", "user1", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/SN001")
                .body(Body::empty())
                .unwrap(),
        )
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
async fn test_device_shows_laptop_details() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-DETAIL-TEST", "detail-host", "172.16.0.1", "detailuser", "2024-01-20T15:30:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/SN-DETAIL-TEST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    assert!(body_str.contains("SN-DETAIL-TEST"), "Should show serial");
    assert!(body_str.contains("detail-host"), "Should show hostname");
    assert!(body_str.contains("172.16.0.1"), "Should show IP");
    assert!(body_str.contains("detailuser"), "Should show user");
}

#[tokio::test]
async fn test_device_shows_drive_details() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let drives_json = serde_json::json!([{
        "device_id": "\\\\.\\PhysicalDrive0",
        "model": "WD Blue 1TB",
        "serial_number": "WD-SERIAL-XYZ",
        "size_bytes": 1000000000000_i64,
        "media_type": "HDD"
    }])
    .to_string();

    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-DRIVE-TEST", "drive-host", "10.0.0.1", "user1", "2024-01-15T10:00:00Z", drives_json],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/SN-DRIVE-TEST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    assert!(body_str.contains("WD Blue 1TB"), "Should show drive model");
    assert!(body_str.contains("WD-SERIAL-XYZ"), "Should show drive serial");
}

#[tokio::test]
async fn test_device_shows_checkin_history() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let conn = rusqlite::Connection::open(db_path).unwrap();

    // Insert laptop
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-HISTORY", "history-host", "10.0.0.1", "user1", "2024-01-20T10:00:00Z", "[]"],
    )
    .unwrap();

    // Insert checkin history
    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-HISTORY", "old-host", "10.0.0.50", "olduser", "2024-01-10T08:00:00Z", "[]"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO checkins (laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["SN-HISTORY", "history-host", "10.0.0.1", "user1", "2024-01-20T10:00:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/SN-HISTORY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    // Both historical entries should appear
    assert!(body_str.contains("2024-01-10"), "Should show old checkin date");
    assert!(body_str.contains("2024-01-20"), "Should show recent checkin date");
}

#[tokio::test]
async fn test_device_url_with_special_characters() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    // Serial with dashes and alphanumeric chars
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute(
        "INSERT INTO laptops (laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["ABC-123-XYZ", "test-laptop", "10.0.0.1", "user1", "2024-01-15T10:00:00Z", "[]"],
    )
    .unwrap();
    drop(conn);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/device/ABC-123-XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
