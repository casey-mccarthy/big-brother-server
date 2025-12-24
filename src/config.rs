use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: String,

    #[serde(default)]
    pub db_path: Option<String>,

    #[serde(default)]
    pub tls_cert: Option<String>,

    #[serde(default)]
    pub tls_key: Option<String>,

    #[serde(default)]
    pub debug: bool,
}

fn default_bind() -> String {
    "0.0.0.0:8443".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            db_path: None,
            tls_cert: None,
            tls_key: None,
            debug: false,
        }
    }
}

/// Get the directory containing the executable
pub fn exe_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("failed to get executable path")?;
    exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("executable has no parent directory"))
}

/// Generate template config.toml if it doesn't exist
fn generate_template_config(config_path: &PathBuf) -> Result<()> {
    let template = r#"# Inventory Server Configuration
#
# This file configures the inventory server REST API.
# Environment variables override these settings:
#   - INVENTORY_BIND
#   - INVENTORY_DB_PATH
#   - INVENTORY_TLS_CERT
#   - INVENTORY_TLS_KEY
#   - INVENTORY_DEBUG

# Server bind address (IP:port)
bind = "0.0.0.0:8443"

# Database file path (defaults to inventory.db in executable directory if not set)
# db_path = "C:\\ProgramData\\InventoryServer\\inventory.db"

# Optional TLS certificate and key paths
# tls_cert = "path/to/cert.pem"
# tls_key = "path/to/key.pem"

# Enable debug mode to log all incoming check-ins
debug = false
"#;

    std::fs::write(config_path, template)
        .with_context(|| format!("failed to write template config to {}", config_path.display()))?;

    println!("Generated template config file: {}", config_path.display());
    Ok(())
}

/// Load config from config.toml in the same directory as the executable.
/// Generates a template file if it doesn't exist.
pub fn load_config() -> Result<Config> {
    let exe_dir = exe_dir()?;
    let config_path = exe_dir.join("config.toml");

    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;
        Ok(config)
    } else {
        // Auto-generate template config file
        generate_template_config(&config_path)?;
        Ok(Config::default())
    }
}

/// Get the default database path (inventory.db in the same directory as the executable)
pub fn default_db_path() -> Result<String> {
    let exe_dir = exe_dir()?;
    let db_path = exe_dir.join("inventory.db");
    Ok(db_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.bind, "0.0.0.0:8443");
        assert_eq!(config.db_path, None);
        assert_eq!(config.tls_cert, None);
        assert_eq!(config.tls_key, None);
        assert_eq!(config.debug, false);
    }

    #[test]
    fn test_default_bind() {
        assert_eq!(default_bind(), "0.0.0.0:8443");
    }

    #[test]
    fn test_toml_parse_minimal() {
        let toml = r#""#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.bind, "0.0.0.0:8443");
        assert_eq!(config.debug, false);
    }

    #[test]
    fn test_toml_parse_full() {
        let toml = r#"
            bind = "127.0.0.1:9000"
            db_path = "/tmp/test.db"
            tls_cert = "/path/to/cert.pem"
            tls_key = "/path/to/key.pem"
            debug = true
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.bind, "127.0.0.1:9000");
        assert_eq!(config.db_path, Some("/tmp/test.db".to_string()));
        assert_eq!(config.tls_cert, Some("/path/to/cert.pem".to_string()));
        assert_eq!(config.tls_key, Some("/path/to/key.pem".to_string()));
        assert_eq!(config.debug, true);
    }

    #[test]
    fn test_toml_parse_partial() {
        let toml = r#"
            bind = "192.168.1.1:8080"
            debug = true
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.bind, "192.168.1.1:8080");
        assert_eq!(config.db_path, None);
        assert_eq!(config.debug, true);
    }

    #[test]
    fn test_exe_dir_success() {
        // This will succeed in test environment
        let result = exe_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_default_db_path_format() {
        let result = default_db_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("inventory.db"));
    }

    #[test]
    fn test_load_config_missing_file() {
        // When config.toml doesn't exist in exe dir, should return defaults
        let result = load_config();
        // Should succeed with defaults (prints warning)
        assert!(result.is_ok());
    }

    #[test]
    fn test_toml_invalid_type() {
        // Test that invalid type for debug field fails gracefully
        let toml = r#"
            debug = "not a boolean"
        "#;
        let result: Result<Config, toml::de::Error> = toml::from_str(toml);
        assert!(result.is_err());
    }
}
