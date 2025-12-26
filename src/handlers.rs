use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusqlite::params;
use std::sync::Arc;
use validator::Validate;

use crate::{
    db,
    errors::CheckInError,
    models::{CheckIn, CheckinRow, Drive, IndexLaptopRow, LaptopRow},
    AppState,
};

// ============== Template Structs ==============

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub laptops: Vec<IndexLaptopRow>,
}

#[derive(Template)]
#[template(path = "device.html")]
pub struct DeviceTemplate {
    pub laptop: LaptopRow,
    pub drives: Vec<Drive>,
    pub checkins: Vec<CheckinRow>,
}

// ============== Web Handlers ==============

/// GET / - Display all laptops
pub async fn index(
    State(state): State<Arc<AppState>>,
) -> Result<IndexTemplate, (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db open: {e}")))?;

    let laptop_rows = db::get_all_laptops(&conn).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("query laptops: {e}"),
        )
    })?;

    // Convert LaptopRow to IndexLaptopRow with parsed drives
    let laptops: Vec<IndexLaptopRow> = laptop_rows
        .into_iter()
        .map(|row| {
            let drives: Vec<Drive> = serde_json::from_str::<Vec<Drive>>(&row.drives_json)
                .unwrap_or_default()
                .into_iter()
                .map(|mut d| {
                    d.device_id = d.device_id.trim_start_matches("\\\\.\\").to_string();
                    d
                })
                .collect();
            let drive_serials_display = {
                let serials: Vec<&str> = drives
                    .iter()
                    .filter_map(|d| d.serial_number.as_deref())
                    .collect();
                if serials.is_empty() {
                    "-".to_string()
                } else {
                    serials.join("<br>")
                }
            };
            IndexLaptopRow {
                laptop_serial: row.laptop_serial,
                hostname: row.hostname,
                ip_address: row.ip_address,
                logged_in_user: row.logged_in_user,
                last_seen_utc: row.last_seen_utc,
                drive_serials_display,
            }
        })
        .collect();

    Ok(IndexTemplate { laptops })
}

/// GET /device/:serial - Display device details and check-in history
pub async fn device_detail(
    State(state): State<Arc<AppState>>,
    Path(serial): Path<String>,
) -> Result<DeviceTemplate, (StatusCode, String)> {
    if state.debug_mode {
        println!(
            "[DEBUG] device_detail called with serial: {:?} (len={})",
            serial,
            serial.len()
        );
        println!("[DEBUG] db_path: {}", state.db_path);
    }

    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db open: {e}")))?;

    // Debug: list all serials in DB
    if state.debug_mode {
        if let Ok(laptops) = db::get_all_laptops(&conn) {
            println!("[DEBUG] All laptops in DB:");
            for l in &laptops {
                println!(
                    "[DEBUG]   serial: {:?} (len={})",
                    l.laptop_serial,
                    l.laptop_serial.len()
                );
            }
        }
    }

    // Fetch laptop
    let laptop = db::get_laptop_by_serial(&conn, &serial)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("query laptop: {e}"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Device not found: {serial}")))?;

    // Parse drives from JSON and clean up device_id (remove \\.\  prefix)
    let drives: Vec<Drive> = serde_json::from_str::<Vec<Drive>>(&laptop.drives_json)
        .unwrap_or_default()
        .into_iter()
        .map(|mut d| {
            d.device_id = d.device_id.trim_start_matches("\\\\.\\").to_string();
            d
        })
        .collect();

    // Fetch check-in history
    let checkins = db::get_checkins_by_serial(&conn, &serial).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("query checkins: {e}"),
        )
    })?;

    Ok(DeviceTemplate {
        laptop,
        drives,
        checkins,
    })
}

// ============== API Handlers ==============

pub async fn checkin(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckIn>,
) -> Result<StatusCode, CheckInError> {
    // Validate input data
    payload.validate()?;

    if state.debug_mode {
        println!(
            "[DEBUG] Checkin received: hostname={}, serial={}, ip={}, user={}, timestamp={}",
            payload.hostname,
            payload.laptop_serial,
            payload.ip_address,
            payload.logged_in_user.as_deref().unwrap_or("(none)"),
            payload.timestamp_utc
        );
        println!("[DEBUG] Drives: {:?}", payload.drives);
    }

    let drives_json = serde_json::to_string(&payload.drives)?;

    // One connection per request is fine for SQLite WAL at this scale.
    let mut conn = rusqlite::Connection::open(&state.db_path).map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            error = ?e,
            "Failed to open database connection"
        );
        CheckInError::DatabaseError(e)
    })?;

    // Ensure pragmas/schema (idempotent). In production, you may do this once at startup.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            error = ?e,
            "Failed to set database pragmas"
        );
        CheckInError::DatabaseError(e)
    })?;

    let tx = conn.transaction().map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            error = ?e,
            "Failed to begin transaction"
        );
        CheckInError::DatabaseError(e)
    })?;

    tx.execute(
        r#"
        INSERT INTO checkins (
            laptop_serial, hostname, ip_address, logged_in_user, timestamp_utc, drives_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            payload.laptop_serial,
            payload.hostname,
            payload.ip_address,
            payload.logged_in_user,
            payload.timestamp_utc,
            drives_json
        ],
    )
    .map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            hostname = %payload.hostname,
            error = ?e,
            "Failed to insert checkin record"
        );
        CheckInError::DatabaseError(e)
    })?;

    tx.execute(
        r#"
        INSERT INTO laptops (
            laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(laptop_serial) DO UPDATE SET
            hostname=excluded.hostname,
            ip_address=excluded.ip_address,
            logged_in_user=excluded.logged_in_user,
            last_seen_utc=excluded.last_seen_utc,
            drives_json=excluded.drives_json
        "#,
        params![
            payload.laptop_serial,
            payload.hostname,
            payload.ip_address,
            payload.logged_in_user,
            payload.timestamp_utc,
            drives_json
        ],
    )
    .map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            hostname = %payload.hostname,
            error = ?e,
            "Failed to upsert laptop record"
        );
        CheckInError::DatabaseError(e)
    })?;

    tx.commit().map_err(|e| {
        tracing::error!(
            laptop_serial = %payload.laptop_serial,
            error = ?e,
            "Failed to commit transaction"
        );
        CheckInError::DatabaseError(e)
    })?;

    Ok(StatusCode::OK)
}
