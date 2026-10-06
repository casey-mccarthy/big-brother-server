# SPEC-server.md — Inventory API Server (Rust) + SQLite WAL

## 1) Overview
The Inventory API Server is a compiled Rust service running on Windows Server that receives check-in payloads and persists:

1) **Current state** keyed by `laptop_serial`  
2) **Historical log** (append-only) of all check-ins

## 2) API Contract
### POST /checkin
- Content-Type: application/json
- Body: see JSON schema below
- Responses:
  - 200 OK — accepted
  - 400 Bad Request — malformed JSON, or a field failed validation
  - 415 Unsupported Media Type — Content-Type is not application/json
  - 422 Unprocessable Entity — JSON does not match the schema (missing field, wrong type)
  - 500 Internal Server Error — DB failure or unexpected runtime error
- Unknown fields are ignored (forward compatibility with newer agents).
- Timestamps: `timestamp_utc` is RFC 3339; the agent sends whole seconds in UTC with a `Z` suffix
  (e.g. `2025-12-18T10:30:00Z`) so stored values are fixed-width and sort lexicographically.

### JSON Schema (informal)
```json
{
  "hostname": "string",
  "ip_address": "string",
  "logged_in_user": "string|null",
  "laptop_serial": "string",
  "drives": [
    {"model":"string","serial_number":"string|null","device_id":"string"}
  ],
  "timestamp_utc": "ISO-8601 string"
}
```

## 3) Database
### SQLite PRAGMAs
Execute at startup (per DB connection or at least once at init):
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
```

### Schema
**Current state**
```sql
CREATE TABLE IF NOT EXISTS laptops (
  laptop_serial TEXT PRIMARY KEY,
  hostname TEXT NOT NULL,
  ip_address TEXT NOT NULL,
  logged_in_user TEXT,
  last_seen_utc TEXT NOT NULL,
  drives_json TEXT NOT NULL
);
```

**Historical log**
```sql
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
```

## 4) Transaction Rules
On each request:
1. BEGIN TRANSACTION
2. INSERT into `checkins`
3. UPSERT into `laptops`
4. COMMIT
If any step fails → ROLLBACK and return 500.

## 5) Deployment
- Run behind Windows Firewall; allow inbound only from managed subnets.
- TLS:
  - Use a certificate bound at the reverse proxy (IIS/nginx) OR direct in Rust (axum + rustls).
  - This skeleton supports rustls directly.
- Storage path:
  - DB file at `C:\ProgramData\InventoryServer\inventory.db` (configurable)

## 6) Acceptance Criteria
- Server starts and listens on configured address/port.
- WAL mode enabled and DB created.
- Receives check-ins and updates both tables.
- Can query current inventory via SQLite tooling.
