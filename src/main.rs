use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

use inventory_server::{config, db, handlers, AppState};

/// Maximum request body size (1 MB)
const MAX_BODY_SIZE: usize = 1024 * 1024;

/// Rate limit: requests per second per IP
const RATE_LIMIT_RPS: u64 = 10;

/// Rate limit burst size
const RATE_LIMIT_BURST: u32 = 20;

#[derive(Parser)]
#[command(name = "inventory-server")]
#[command(about = "REST API server for endpoint inventory data")]
struct Args {
    /// Enable debug mode to log all incoming checkins
    #[arg(short, long)]
    debug: bool,
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }
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

    let db_path = match std::env::var("INVENTORY_DB_PATH")
        .ok()
        .or(cfg.db_path.clone())
    {
        Some(path) => path,
        None => config::default_db_path()?,
    };

    // Debug mode can be enabled via --debug flag, INVENTORY_DEBUG env var, or config file
    let debug_mode = args.debug
        || std::env::var("INVENTORY_DEBUG")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
        || cfg.debug;

    println!(
        "Starting inventory-server v{} on {}",
        env!("CARGO_PKG_VERSION"),
        bind_addr
    );
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

    let state = Arc::new(AppState {
        db_path,
        debug_mode,
    });

    // Configure rate limiting (per IP)
    let governor_config = GovernorConfigBuilder::default()
        .per_second(RATE_LIMIT_RPS)
        .burst_size(RATE_LIMIT_BURST)
        .finish()
        .context("failed to build rate limiter config")?;

    let governor_limiter = governor_config.limiter().clone();
    let rate_limit_layer = GovernorLayer {
        config: Arc::new(governor_config),
    };

    // Start background task to clean up rate limiter state
    let interval = Duration::from_secs(60);
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        governor_limiter.retain_recent();
    });

    // Build the router with all middleware
    let app = Router::new()
        // Health check endpoints (no rate limiting)
        .route("/health", get(handlers::health))
        .route("/ready", get(handlers::ready))
        // Main application routes
        .route("/", get(handlers::index))
        .route("/device/:serial", get(handlers::device_detail))
        .route("/checkin", post(handlers::checkin))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
                .layer(rate_limit_layer),
        )
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
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .context("load tls cert/key")?;

        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();

        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.graceful_shutdown(Some(Duration::from_secs(30)));
        });

        axum_server::bind_rustls(bind_addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .context("serve rustls")?;
    } else {
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();

        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.graceful_shutdown(Some(Duration::from_secs(30)));
        });

        axum_server::bind(bind_addr)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .context("serve http")?;
    }

    tracing::info!("Server shutdown complete");
    Ok(())
}
