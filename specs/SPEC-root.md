# SPEC-root.md — Endpoint Inventory Collection System (Package Entry Point)

## Purpose
A Rust-based endpoint agent runs as a Windows Service on laptops to collect asset inventory data and report to a Rust API server. The server persists both current state and historical logs in a single SQLite database configured with WAL.

## Deliverables in This Package
- `inventory-agent/` — Rust Windows service agent
- `inventory-server/` — Rust API server with SQLite WAL persistence
- `deployment/` — PowerShell scripts for remote installation and OU rollout
- `specs/` — Detailed specs and diagrams

## Diagrams
See `specs/diagrams.mmd` for Mermaid diagrams.
