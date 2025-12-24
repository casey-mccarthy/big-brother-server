use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Drive {
    #[validate(length(min = 1, max = 256), custom(function = "validate_printable_ascii_required"))]
    pub model: String,
    #[validate(length(max = 256), custom(function = "validate_printable_ascii_required"))]
    pub serial_number: Option<String>,
    #[validate(length(min = 1, max = 256), custom(function = "validate_printable_ascii_required"))]
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CheckIn {
    #[validate(length(min = 1, max = 63), custom(function = "validate_hostname"))]
    pub hostname: String,
    #[validate(custom(function = "validate_ip_address"))]
    pub ip_address: String,
    #[validate(length(max = 512), custom(function = "validate_printable_ascii_required"))]
    pub logged_in_user: Option<String>,
    #[validate(length(min = 1, max = 128), custom(function = "validate_printable_ascii_required"))]
    pub laptop_serial: String,
    #[validate(length(max = 32), nested)]
    pub drives: Vec<Drive>,
    #[validate(custom(function = "validate_timestamp"))]
    pub timestamp_utc: String,
}

/// Represents a row from the laptops table for display
#[derive(Debug)]
pub struct LaptopRow {
    pub laptop_serial: String,
    pub hostname: String,
    pub ip_address: String,
    pub logged_in_user: Option<String>,
    pub last_seen_utc: String,
    pub drives_json: String,
}

/// Represents a row from the checkins table for display
#[derive(Debug)]
pub struct CheckinRow {
    pub hostname: String,
    pub ip_address: String,
    pub logged_in_user: Option<String>,
    pub timestamp_utc: String,
}

/// Represents a laptop row with parsed drives for index page display
#[derive(Debug)]
pub struct IndexLaptopRow {
    pub laptop_serial: String,
    pub hostname: String,
    pub ip_address: String,
    pub logged_in_user: Option<String>,
    pub last_seen_utc: String,
    pub drive_serials_display: String,
}

/// Validates that a string is a valid IPv4 or IPv6 address
fn validate_ip_address(ip: &str) -> Result<(), ValidationError> {
    use std::str::FromStr;
    std::net::IpAddr::from_str(ip)
        .map(|_| ())
        .map_err(|_| ValidationError::new("invalid_ip"))
}

/// Validates that a string is a valid RFC3339 timestamp
fn validate_timestamp(ts: &str) -> Result<(), ValidationError> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|_| ())
        .map_err(|_| ValidationError::new("invalid_timestamp"))
}

/// Validates that a string contains only printable ASCII characters
/// Works for both required and optional fields (called on inner String when Option is Some)
fn validate_printable_ascii_required(s: &str) -> Result<(), ValidationError> {
    if s.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_characters"))
    }
}

/// Validates that a hostname follows Windows computer name conventions
/// Allows alphanumeric, hyphens, and underscores (common in Windows environments)
fn validate_hostname(hostname: &str) -> Result<(), ValidationError> {
    // Must start and end with alphanumeric, can contain hyphens and underscores in the middle
    // This accommodates real-world Windows computer names which may include underscores
    let valid = hostname
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && hostname.chars().next().map_or(false, |c| c.is_ascii_alphanumeric())
        && hostname.chars().last().map_or(false, |c| c.is_ascii_alphanumeric());

    if valid {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_hostname"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_serialization() {
        let drive = Drive {
            model: "Samsung SSD 970".to_string(),
            serial_number: Some("S5H2NS0N123456".to_string()),
            device_id: r"\\.\PHYSICALDRIVE0".to_string(),
        };

        let json = serde_json::to_string(&drive).unwrap();
        assert!(json.contains("Samsung SSD 970"));
        assert!(json.contains("S5H2NS0N123456"));
        assert!(json.contains("PHYSICALDRIVE0"));
    }

    #[test]
    fn test_drive_deserialization() {
        let json = r#"{
            "model": "WD Blue",
            "serial_number": "WD-123456",
            "device_id": "PHYSICALDRIVE1"
        }"#;

        let drive: Drive = serde_json::from_str(json).unwrap();
        assert_eq!(drive.model, "WD Blue");
        assert_eq!(drive.serial_number, Some("WD-123456".to_string()));
        assert_eq!(drive.device_id, "PHYSICALDRIVE1");
    }

    #[test]
    fn test_drive_null_serial() {
        let json = r#"{
            "model": "Generic Drive",
            "serial_number": null,
            "device_id": "PHYSICALDRIVE2"
        }"#;

        let drive: Drive = serde_json::from_str(json).unwrap();
        assert_eq!(drive.serial_number, None);
    }

    #[test]
    fn test_checkin_full_payload() {
        let checkin = CheckIn {
            hostname: "LAPTOP-TEST".to_string(),
            ip_address: "192.168.1.100".to_string(),
            logged_in_user: Some("DOMAIN\\user".to_string()),
            laptop_serial: "ABC123XYZ".to_string(),
            drives: vec![
                Drive {
                    model: "Samsung SSD".to_string(),
                    serial_number: Some("S123456".to_string()),
                    device_id: "PHYSICALDRIVE0".to_string(),
                }
            ],
            timestamp_utc: "2025-12-18T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&checkin).unwrap();
        let parsed: CheckIn = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.hostname, "LAPTOP-TEST");
        assert_eq!(parsed.laptop_serial, "ABC123XYZ");
        assert_eq!(parsed.drives.len(), 1);
    }

    #[test]
    fn test_checkin_no_user() {
        let json = r#"{
            "hostname": "LAPTOP-01",
            "ip_address": "10.0.0.5",
            "logged_in_user": null,
            "laptop_serial": "SERIAL001",
            "drives": [],
            "timestamp_utc": "2025-12-18T12:00:00Z"
        }"#;

        let checkin: CheckIn = serde_json::from_str(json).unwrap();
        assert_eq!(checkin.logged_in_user, None);
        assert_eq!(checkin.drives.len(), 0);
    }

    #[test]
    fn test_checkin_multiple_drives() {
        let checkin = CheckIn {
            hostname: "WORKSTATION".to_string(),
            ip_address: "172.16.0.10".to_string(),
            logged_in_user: Some("admin".to_string()),
            laptop_serial: "MULTI-DRIVE-001".to_string(),
            drives: vec![
                Drive {
                    model: "Drive1".to_string(),
                    serial_number: Some("SN1".to_string()),
                    device_id: "PHYSICALDRIVE0".to_string(),
                },
                Drive {
                    model: "Drive2".to_string(),
                    serial_number: None,
                    device_id: "PHYSICALDRIVE1".to_string(),
                },
            ],
            timestamp_utc: "2025-12-18T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&checkin).unwrap();
        let parsed: CheckIn = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.drives.len(), 2);
        assert_eq!(parsed.drives[1].serial_number, None);
    }

    #[test]
    fn test_hostname_validation_with_underscores() {
        use validator::Validate;

        // Valid: underscores in middle
        let checkin = CheckIn {
            hostname: "LAPTOP_TEST_01".to_string(),
            ip_address: "192.168.1.100".to_string(),
            logged_in_user: Some("user".to_string()),
            laptop_serial: "ABC123".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T10:00:00Z".to_string(),
        };
        assert!(checkin.validate().is_ok());

        // Valid: mixed separators
        let checkin2 = CheckIn {
            hostname: "WIN_DESKTOP-01".to_string(),
            ip_address: "192.168.1.101".to_string(),
            logged_in_user: None,
            laptop_serial: "XYZ789".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T11:00:00Z".to_string(),
        };
        assert!(checkin2.validate().is_ok());

        // Invalid: starts with underscore
        let checkin3 = CheckIn {
            hostname: "_INVALID".to_string(),
            ip_address: "192.168.1.102".to_string(),
            logged_in_user: None,
            laptop_serial: "BAD001".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T12:00:00Z".to_string(),
        };
        assert!(checkin3.validate().is_err());

        // Invalid: ends with underscore
        let checkin4 = CheckIn {
            hostname: "INVALID_".to_string(),
            ip_address: "192.168.1.103".to_string(),
            logged_in_user: None,
            laptop_serial: "BAD002".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T13:00:00Z".to_string(),
        };
        assert!(checkin4.validate().is_err());

        // Invalid: contains special characters
        let checkin5 = CheckIn {
            hostname: "HOST@NAME".to_string(),
            ip_address: "192.168.1.104".to_string(),
            logged_in_user: None,
            laptop_serial: "BAD003".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T14:00:00Z".to_string(),
        };
        assert!(checkin5.validate().is_err());
    }

    #[test]
    fn test_hostname_validation_traditional_formats() {
        use validator::Validate;

        // Traditional DNS-style hostname (still valid)
        let checkin = CheckIn {
            hostname: "LAPTOP-TEST".to_string(),
            ip_address: "192.168.1.200".to_string(),
            logged_in_user: None,
            laptop_serial: "TRAD001".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T15:00:00Z".to_string(),
        };
        assert!(checkin.validate().is_ok());

        // Simple alphanumeric
        let checkin2 = CheckIn {
            hostname: "WORKSTATION01".to_string(),
            ip_address: "192.168.1.201".to_string(),
            logged_in_user: None,
            laptop_serial: "TRAD002".to_string(),
            drives: vec![],
            timestamp_utc: "2025-12-21T16:00:00Z".to_string(),
        };
        assert!(checkin2.validate().is_ok());
    }
}
