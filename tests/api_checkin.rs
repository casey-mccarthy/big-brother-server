mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use inventory_server::db;
use tower::ServiceExt;

#[tokio::test]
async fn test_valid_checkin_returns_200() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::valid_checkin_json()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_checkin_persists_to_laptops_table() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let _response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::valid_checkin_json()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify data was persisted
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let laptops = db::get_all_laptops(&conn).unwrap();

    assert_eq!(laptops.len(), 1);
    assert_eq!(laptops[0].laptop_serial, "SN123456789");
    assert_eq!(laptops[0].hostname, "TEST-LAPTOP-001");
}

#[tokio::test]
async fn test_checkin_persists_to_checkins_table() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap();

    let _response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::valid_checkin_json()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify checkin was recorded
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let checkins = db::get_checkins_by_serial(&conn, "SN123456789").unwrap();

    assert_eq!(checkins.len(), 1);
    assert_eq!(checkins[0].hostname, "TEST-LAPTOP-001");
}

#[tokio::test]
async fn test_multiple_checkins_updates_laptop() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap().to_string();

    // First checkin - clone the router for reuse
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::checkin_json_with(
                    "hostname-v1",
                    "SN001",
                    "192.168.1.1",
                    Some("user1"),
                    "2024-01-10T10:00:00Z",
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second checkin with updated info
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::checkin_json_with(
                    "hostname-v2",
                    "SN001",
                    "192.168.1.2",
                    Some("user2"),
                    "2024-01-15T10:00:00Z",
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify laptop table was updated (UPSERT)
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let laptops = db::get_all_laptops(&conn).unwrap();

    assert_eq!(laptops.len(), 1, "Should still have only one laptop entry");
    assert_eq!(
        laptops[0].hostname, "hostname-v2",
        "Hostname should be updated"
    );
    assert_eq!(laptops[0].ip_address, "192.168.1.2", "IP should be updated");
}

#[tokio::test]
async fn test_multiple_checkins_appends_history() {
    let (app, temp_db) = common::setup_test_app();
    let db_path = temp_db.path().to_str().unwrap().to_string();

    // First checkin
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(common::checkin_json_with(
                    "hostname-v1",
                    "SN001",
                    "192.168.1.1",
                    Some("user1"),
                    "2024-01-10T10:00:00Z",
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    // Second checkin
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/checkin")
            .header("content-type", "application/json")
            .body(Body::from(common::checkin_json_with(
                "hostname-v2",
                "SN001",
                "192.168.1.2",
                Some("user2"),
                "2024-01-15T10:00:00Z",
            )))
            .unwrap(),
    )
    .await
    .unwrap();

    // Verify checkins table has both entries (audit trail)
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let checkins = db::get_checkins_by_serial(&conn, "SN001").unwrap();

    assert_eq!(checkins.len(), 2, "Should have two checkin records");
}

#[tokio::test]
async fn test_checkin_with_empty_drives() {
    let (app, _temp_db) = common::setup_test_app();

    let payload = common::checkin_json_with(
        "test-host",
        "SN001",
        "10.0.0.1",
        None,
        "2024-01-15T10:00:00Z",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_checkin_without_logged_in_user() {
    let (app, _temp_db) = common::setup_test_app();

    let payload = common::checkin_json_with(
        "test-host",
        "SN001",
        "10.0.0.1",
        None, // No user logged in
        "2024-01-15T10:00:00Z",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_hostname_returns_400() {
    let (app, _temp_db) = common::setup_test_app();

    // Hostname with invalid characters (spaces)
    let payload = serde_json::json!({
        "hostname": "invalid hostname with spaces",
        "laptop_serial": "SN001",
        "ip_address": "192.168.1.1",
        "logged_in_user": null,
        "timestamp_utc": "2024-01-15T10:00:00Z",
        "drives": []
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 400 for validation errors
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_invalid_ip_address_returns_400() {
    let (app, _temp_db) = common::setup_test_app();

    let payload = serde_json::json!({
        "hostname": "valid-hostname",
        "laptop_serial": "SN001",
        "ip_address": "not-an-ip-address",
        "logged_in_user": null,
        "timestamp_utc": "2024-01-15T10:00:00Z",
        "drives": []
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 400 for validation errors
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_missing_required_fields_returns_422() {
    let (app, _temp_db) = common::setup_test_app();

    // Missing laptop_serial
    let payload = serde_json::json!({
        "hostname": "valid-hostname",
        "ip_address": "192.168.1.1",
        "logged_in_user": null,
        "timestamp_utc": "2024-01-15T10:00:00Z",
        "drives": []
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for missing required fields (deserialization error)
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_invalid_json_returns_400() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from("{ invalid json }"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 400 for invalid JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
