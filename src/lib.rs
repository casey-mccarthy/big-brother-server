// Library exports for integration tests

pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod models;

pub struct AppState {
    pub db_path: String,
    pub debug_mode: bool,
}
