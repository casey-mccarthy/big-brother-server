use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::models::{CheckinRow, LaptopRow};

pub fn open_and_init(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path).context("open sqlite db failed")?;

    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS laptops (
          laptop_serial TEXT PRIMARY KEY,
          hostname TEXT NOT NULL,
          ip_address TEXT NOT NULL,
          logged_in_user TEXT,
          last_seen_utc TEXT NOT NULL,
          drives_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS checkins (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          laptop_serial TEXT NOT NULL,
          hostname TEXT NOT NULL,
          ip_address TEXT NOT NULL,
          logged_in_user TEXT,
          timestamp_utc TEXT NOT NULL,
          drives_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_checkins_laptop_serial ON checkins(laptop_serial);
        CREATE INDEX IF NOT EXISTS idx_checkins_timestamp ON checkins(timestamp_utc);
        "#,
    )
    .context("db init batch failed")?;

    Ok(conn)
}

/// Fetch all laptops ordered by last_seen_utc descending (most recent first)
pub fn get_all_laptops(conn: &Connection) -> Result<Vec<LaptopRow>> {
    let mut stmt = conn.prepare(
        "SELECT laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json
         FROM laptops
         ORDER BY last_seen_utc DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(LaptopRow {
            laptop_serial: row.get(0)?,
            hostname: row.get(1)?,
            ip_address: row.get(2)?,
            logged_in_user: row.get(3)?,
            last_seen_utc: row.get(4)?,
            drives_json: row.get(5)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>()
        .context("fetch all laptops")
}

/// Fetch a single laptop by serial number
pub fn get_laptop_by_serial(conn: &Connection, serial: &str) -> Result<Option<LaptopRow>> {
    let mut stmt = conn.prepare(
        "SELECT laptop_serial, hostname, ip_address, logged_in_user, last_seen_utc, drives_json
         FROM laptops
         WHERE laptop_serial = ?1",
    )?;

    let mut rows = stmt.query_map([serial], |row| {
        Ok(LaptopRow {
            laptop_serial: row.get(0)?,
            hostname: row.get(1)?,
            ip_address: row.get(2)?,
            logged_in_user: row.get(3)?,
            last_seen_utc: row.get(4)?,
            drives_json: row.get(5)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// Fetch check-in history for a specific laptop, ordered by timestamp descending
pub fn get_checkins_by_serial(conn: &Connection, serial: &str) -> Result<Vec<CheckinRow>> {
    let mut stmt = conn.prepare(
        "SELECT hostname, ip_address, logged_in_user, timestamp_utc
         FROM checkins
         WHERE laptop_serial = ?1
         ORDER BY timestamp_utc DESC",
    )?;

    let rows = stmt.query_map([serial], |row| {
        Ok(CheckinRow {
            hostname: row.get(0)?,
            ip_address: row.get(1)?,
            logged_in_user: row.get(2)?,
            timestamp_utc: row.get(3)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>()
        .context("fetch checkins by serial")
}
