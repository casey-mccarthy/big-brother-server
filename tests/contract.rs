//! Wire-contract tests between inventory-agent and inventory-server.
//!
//! `tests/fixtures/checkin.json` is the canonical check-in payload. The same file
//! is committed to the inventory-agent repository, where a test asserts the agent
//! serializes exactly this JSON. Here we assert the server accepts exactly this
//! JSON. If either side changes the schema, one of the two suites goes red.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use inventory_server::models::CheckIn;
use serde_json::Value;
use tower::ServiceExt;
use validator::Validate;

const FIXTURE: &str = include_str!("fixtures/checkin.json");

fn fixture_value() -> Value {
    serde_json::from_str(FIXTURE).expect("fixture is valid JSON")
}

#[test]
fn fixture_matches_server_model_exactly() {
    let checkin: CheckIn = serde_json::from_str(FIXTURE).expect("fixture deserializes");
    checkin.validate().expect("fixture passes validation");

    // Re-serializing must reproduce the fixture byte-for-byte as JSON values.
    // This fails if the server struct gains, loses, or renames a field relative
    // to what the agent sends, including the `null` drive serial.
    let roundtrip = serde_json::to_value(&checkin).unwrap();
    assert_eq!(roundtrip, fixture_value());
}

#[tokio::test]
async fn fixture_is_accepted_and_rendered_end_to_end() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(FIXTURE))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/device/ABC123XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&body);
    assert!(body.contains("LAPTOP-ABC123"));
    assert!(body.contains("Samsung SSD 970 EVO 500GB"));
    assert!(body.contains("S4EVNX0M123456"));
    // The `\\.\` device prefix is stripped for display.
    assert!(body.contains("PHYSICALDRIVE1"));
    assert!(!body.contains(r"\\.\PHYSICALDRIVE1"));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&body);
    assert!(body.contains("ABC123XYZ"));
    assert!(body.contains("S4EVNX0M123456"));
}

#[tokio::test]
async fn unknown_fields_are_ignored_for_forward_compatibility() {
    // A newer agent may send fields this server does not know yet. The server
    // must keep accepting the check-in rather than rejecting the whole fleet.
    let (app, _temp_db) = common::setup_test_app();

    let mut payload = fixture_value();
    payload["future_field"] = Value::from("ignored");
    payload["drives"][0]["size_bytes"] = Value::from(500_107_862_016_i64);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn wrong_content_type_is_rejected() {
    let (app, _temp_db) = common::setup_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/checkin")
                .header("content-type", "text/plain")
                .body(Body::from(FIXTURE))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
