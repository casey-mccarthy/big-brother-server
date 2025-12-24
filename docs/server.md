# Inventory Server Guide

The inventory server is a REST API server that receives check-in data from agents and provides a web interface to view the inventory.

## Building

```powershell
cd inventory-server
cargo build --release
```

The compiled binary will be at `target/release/inventory-server.exe`.

## Configuration

The server can be configured via three methods (in order of precedence):

1. **Command-line flags** (highest priority)
2. **Environment variables**
3. **config.toml file** (lowest priority)

### config.toml

Place a `config.toml` file in the same directory as the executable:

```toml
# Bind address and port (default: 0.0.0.0:8443)
bind = "0.0.0.0:8443"

# Database path (default: inventory.db next to executable)
db_path = "C:\\ProgramData\\InventoryServer\\inventory.db"

# Enable debug mode to log incoming checkins (default: false)
debug = false

# TLS certificate and key paths (optional, leave commented for HTTP)
# tls_cert = "cert.pem"
# tls_key = "key.pem"
```

### Environment Variables

Environment variables override config file values:

| Variable | Description | Default |
|----------|-------------|---------|
| `INVENTORY_BIND` | Address and port to bind | `0.0.0.0:8443` |
| `INVENTORY_DB_PATH` | Path to SQLite database file | `inventory.db` (next to exe) |
| `INVENTORY_DEBUG` | Enable debug logging (`true` or `1`) | `false` |
| `INVENTORY_TLS_CERT` | Path to TLS certificate (PEM format) | (none) |
| `INVENTORY_TLS_KEY` | Path to TLS private key (PEM format) | (none) |
| `RUST_LOG` | Logging level (e.g., `info`, `debug`) | (none) |

### Command-Line Flags

```
inventory-server.exe [OPTIONS]

Options:
  -d, --debug    Enable debug mode to log all incoming checkins
  -h, --help     Print help
```

## Running

### HTTP Mode (Development)

```powershell
# Using defaults
.\inventory-server.exe

# With environment variables
$env:INVENTORY_BIND = "0.0.0.0:8080"
$env:INVENTORY_DB_PATH = "C:\ProgramData\InventoryServer\inventory.db"
.\inventory-server.exe
```

### HTTPS Mode (Production)

```powershell
# Via config.toml
# tls_cert = "C:\\certs\\server.pem"
# tls_key = "C:\\certs\\server-key.pem"

# Or via environment variables
$env:INVENTORY_TLS_CERT = "C:\certs\server.pem"
$env:INVENTORY_TLS_KEY = "C:\certs\server-key.pem"
.\inventory-server.exe
```

### Debug Mode

Debug mode logs all incoming check-in payloads to the console. Enable via any of:

```powershell
# Command-line flag
.\inventory-server.exe --debug

# Environment variable
$env:INVENTORY_DEBUG = "true"
.\inventory-server.exe

# config.toml
# debug = true
```

## Web Interface

### Index Page (`/`)

Displays a table of all inventoried devices with:
- Laptop serial number (links to detail page)
- Hostname
- IP address
- Logged-in user
- Last seen timestamp
- Drive serial numbers

Devices are sorted by most recently seen.

### Device Detail Page (`/device/:serial`)

Shows detailed information for a specific device:
- Current device information (hostname, IP, user, serial)
- List of physical drives with model and serial number
- Check-in history showing all previous check-ins

## API Reference

### POST /checkin

Receives inventory data from agents.

**Request:**
```
Content-Type: application/json
```

**Request Body:**
```json
{
  "hostname": "LAPTOP-ABC123",
  "ip_address": "192.168.1.100",
  "logged_in_user": "DOMAIN\\jsmith",
  "laptop_serial": "ABC123XYZ",
  "drives": [
    {
      "model": "Samsung SSD 970 EVO 500GB",
      "serial_number": "S4EVNX0M123456",
      "device_id": "\\\\.\\PHYSICALDRIVE0"
    }
  ],
  "timestamp_utc": "2024-01-15T10:30:00Z"
}
```

**Response Codes:**
| Code | Description |
|------|-------------|
| 200 | Check-in accepted |
| 400 | Invalid JSON or missing required fields |
| 500 | Database or server error |

## Database Schema

The server uses SQLite with WAL (Write-Ahead Logging) mode for better concurrent access.

### Tables

**laptops** - Current state (one row per device)
```sql
CREATE TABLE laptops (
  laptop_serial TEXT PRIMARY KEY,
  hostname TEXT NOT NULL,
  ip_address TEXT NOT NULL,
  logged_in_user TEXT,
  last_seen_utc TEXT NOT NULL,
  drives_json TEXT NOT NULL
);
```

**checkins** - Historical audit trail (append-only)
```sql
CREATE TABLE checkins (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  laptop_serial TEXT NOT NULL,
  hostname TEXT NOT NULL,
  ip_address TEXT NOT NULL,
  logged_in_user TEXT,
  timestamp_utc TEXT NOT NULL,
  drives_json TEXT NOT NULL
);

CREATE INDEX idx_checkins_laptop_serial ON checkins(laptop_serial);
CREATE INDEX idx_checkins_timestamp ON checkins(timestamp_utc);
```

### Transaction Behavior

Each check-in is processed in a single transaction:
1. INSERT into `checkins` (historical record)
2. UPSERT into `laptops` (update current state)
3. COMMIT

If either operation fails, the entire transaction is rolled back.

## Production Deployment

### Recommended Configuration

1. Run behind Windows Firewall, allowing inbound only from managed subnets
2. Use TLS (either directly or via reverse proxy)
3. Store database in a dedicated directory with appropriate permissions

### Example Production Setup

```powershell
# Create directories
New-Item -ItemType Directory -Path "C:\ProgramData\InventoryServer" -Force

# Copy binary and config
Copy-Item inventory-server.exe "C:\ProgramData\InventoryServer\"
Copy-Item config.toml "C:\ProgramData\InventoryServer\"

# Create config.toml
@"
bind = "0.0.0.0:8443"
db_path = "C:\\ProgramData\\InventoryServer\\inventory.db"
tls_cert = "C:\\ProgramData\\InventoryServer\\cert.pem"
tls_key = "C:\\ProgramData\\InventoryServer\\key.pem"
"@ | Out-File "C:\ProgramData\InventoryServer\config.toml"

# Run (consider setting up as a Windows Service)
& "C:\ProgramData\InventoryServer\inventory-server.exe"
```

### Running as a Windows Service

To run the server as a Windows Service, use a service wrapper like [NSSM](https://nssm.cc/):

```powershell
nssm install InventoryServer "C:\ProgramData\InventoryServer\inventory-server.exe"
nssm set InventoryServer AppDirectory "C:\ProgramData\InventoryServer"
nssm start InventoryServer
```
