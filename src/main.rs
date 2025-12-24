use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use inventory_server::{config, db, handlers, AppState};

#[derive(Parser)]
#[command(name = "inventory-server")]
#[command(about = "REST API server for endpoint inventory data")]
struct Args {
    /// Enable debug mode to log all incoming checkins
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load config from config.toml in the same directory as the executable
    let cfg = config::load_config()?;

    // Environment variables override config file values
    let bind_addr: SocketAddr = std::env::var("INVENTORY_BIND")
        .unwrap_or(cfg.bind)
        .parse()
        .context("parse bind address")?;

    let db_path = match std::env::var("INVENTORY_DB_PATH").ok().or(cfg.db_path.clone()) {
        Some(path) => path,
        None => config::default_db_path()?,
    };

    // Debug mode can be enabled via --debug flag, INVENTORY_DEBUG env var, or config file
    let debug_mode = args.debug
        || std::env::var("INVENTORY_DEBUG")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
        || cfg.debug;

    println!("Starting inventory-server on {}", bind_addr);
    println!("Database path: {}", db_path);
    if debug_mode {
        println!("[DEBUG] Debug mode enabled - will log all incoming checkins");
    }

    // Ensure DB directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Initialize schema + WAL
    let _ = db::open_and_init(&db_path)?;

    let state = Arc::new(AppState { db_path, debug_mode });

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/device/:serial", get(handlers::device_detail))
        .route("/checkin", post(handlers::checkin))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // TLS config: env vars override config file
    let cert_path = std::env::var("INVENTORY_TLS_CERT")
        .ok()
        .or(cfg.tls_cert)
        .unwrap_or_default();
    let key_path = std::env::var("INVENTORY_TLS_KEY")
        .ok()
        .or(cfg.tls_key)
        .unwrap_or_default();

    if !cert_path.is_empty() && !key_path.is_empty() {
        let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .context("load tls cert/key")?;
        axum_server::bind_rustls(bind_addr, config)
            .serve(app.into_make_service())
            .await
            .context("serve rustls")?;
    } else {
        axum_server::bind(bind_addr)
            .serve(app.into_make_service())
            .await
            .context("serve http")?;
    }

    Ok(())
}
