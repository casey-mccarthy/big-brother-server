# inventory-server

REST API server for endpoint inventory with SQLite persistence. Part of the Big Brother endpoint inventory system.

## Overview

The inventory-server receives check-in data from inventory-agent instances running on endpoint machines and persists the data to a SQLite database with WAL mode enabled for performance.

### Features
- REST API endpoint for agent check-ins (`POST /checkin`)
- Web dashboard for viewing inventory (`GET /`)
- Device detail pages (`GET /device/:serial`)
- SQLite with WAL mode for concurrent reads
- Optional TLS termination

### Database Schema
- **laptops**: Current state keyed by laptop_serial (UPSERT on conflict)
- **checkins**: Historical audit trail with auto-increment ID, indexed by laptop_serial and timestamp_utc

## Build

On a Windows build host with Rust toolchain:

```powershell
cargo build --release
```

Output: `target\release\inventory-server.exe`

## Configuration

Environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `INVENTORY_BIND` | No | `0.0.0.0:8443` | Server bind address and port |
| `INVENTORY_DB_PATH` | No | `C:\ProgramData\InventoryServer\inventory.db` | SQLite database path |
| `INVENTORY_TLS_CERT` | No | - | Path to PEM certificate (enables TLS) |
| `INVENTORY_TLS_KEY` | No | - | Path to PEM private key (enables TLS) |

## Running

### Basic (HTTP)
```powershell
$env:INVENTORY_BIND="0.0.0.0:8443"
$env:INVENTORY_DB_PATH="C:\ProgramData\InventoryServer\inventory.db"
.\inventory-server.exe
```

### With TLS
```powershell
$env:INVENTORY_BIND="0.0.0.0:8443"
$env:INVENTORY_DB_PATH="C:\ProgramData\InventoryServer\inventory.db"
$env:INVENTORY_TLS_CERT="path\to\cert.pem"
$env:INVENTORY_TLS_KEY="path\to\key.pem"
.\inventory-server.exe
```

If you terminate TLS upstream (IIS/nginx), run server on HTTP internally and enforce firewall rules.

## API Endpoints

### POST /checkin
Receives inventory data from agents.

Request body:
```json
{
  "hostname": "LAPTOP01",
  "ip_address": "192.168.1.100",
  "logged_in_user": "jsmith",
  "laptop_serial": "ABC123",
  "drives": [
    {
      "model": "Samsung SSD 970 EVO",
      "serial_number": "S4EVNF0M123456",
      "device_id": "\\\\.\\PHYSICALDRIVE0"
    }
  ],
  "timestamp_utc": "2024-01-15T10:30:00Z"
}
```

### GET /
Web dashboard showing all inventoried laptops.

### GET /device/:serial
Device detail page for a specific laptop serial.

## Architecture

```
src/
├── main.rs      # Axum HTTP server setup, TLS configuration
├── handlers.rs  # POST /checkin endpoint and web UI routes
├── db.rs        # Schema initialization with WAL mode
├── models.rs    # CheckIn, Drive structs with validation
├── config.rs    # Configuration handling
└── errors.rs    # Error types

templates/
├── base.html    # Base template
├── index.html   # Dashboard view
└── device.html  # Device detail view
```

### Data Flow
1. Agent POSTs JSON to `/checkin`
2. Server validates and deserializes payload
3. Server writes to both tables in a transaction:
   - `checkins`: Append-only audit trail
   - `laptops`: Current state (UPSERT by laptop_serial)

## Platform Requirements

- Windows recommended for production deployment
- Can build and run on Linux/macOS for development
- SQLite WAL mode requires filesystem that supports it

## License

MIT
