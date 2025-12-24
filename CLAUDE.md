# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **inventory-server** component of the Big Brother endpoint inventory system. It is a REST API server that receives check-in data from inventory-agent instances and persists it to SQLite.

## Build & Run Commands

Build:
```powershell
cargo build --release
```

Run locally:
```powershell
$env:INVENTORY_BIND="0.0.0.0:8443"
$env:INVENTORY_DB_PATH="C:\ProgramData\InventoryServer\inventory.db"
.\target\release\inventory-server.exe
```

Optional TLS configuration:
```powershell
$env:INVENTORY_TLS_CERT="path\to\cert.pem"
$env:INVENTORY_TLS_KEY="path\to\key.pem"
```

## Architecture

### Source Files (src/)
- **main.rs**: Axum HTTP server setup, TLS configuration, AppState initialization
- **handlers.rs**: POST /checkin endpoint - transactional insert to both tables, web UI routes
- **db.rs**: Schema initialization with WAL mode pragmas
- **models.rs**: CheckIn and Drive structs with validation (deserialization from agent)
- **config.rs**: Configuration handling
- **errors.rs**: Error types

### Templates (templates/)
- **base.html**: Base Askama template
- **index.html**: Dashboard showing all laptops
- **device.html**: Device detail view

### Database Schema (SQLite with WAL)
- **laptops**: Current state keyed by laptop_serial (UPSERT on conflict)
- **checkins**: Historical audit trail with auto-increment ID, indexed by laptop_serial and timestamp_utc

### Data Flow
1. Agent POSTs JSON to /checkin endpoint
2. Server validates payload (handlers.rs)
3. Server writes to both tables in transaction (db.rs):
   - checkins: append-only audit trail
   - laptops: current state (UPSERT by laptop_serial)

### Configuration
Environment variables:
- `INVENTORY_BIND` (default `0.0.0.0:8443`)
- `INVENTORY_DB_PATH` (default `C:\ProgramData\InventoryServer\inventory.db`)
- `INVENTORY_TLS_CERT` (optional, path to PEM cert)
- `INVENTORY_TLS_KEY` (optional, path to PEM key)

The server opens one connection per request (acceptable for SQLite WAL at this scale). WAL mode is enabled both at startup and per-request for robustness.

## Platform Requirements
- Can build on Windows, Linux, or macOS
- Production deployment typically on Windows
- SQLite WAL mode requires compatible filesystem
