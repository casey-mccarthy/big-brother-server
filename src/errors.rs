use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Error types for the checkin endpoint
#[derive(Debug)]
pub enum CheckInError {
    /// Validation failed on input data
    ValidationFailed(validator::ValidationErrors),
    /// Database operation error
    DatabaseError(rusqlite::Error),
    /// JSON serialization error
    SerializationError(serde_json::Error),
}

impl IntoResponse for CheckInError {
    fn into_response(self) -> Response {
        match self {
            Self::ValidationFailed(e) => {
                // Log detailed validation errors internally
                tracing::warn!(
                    validation_errors = ?e,
                    "Input validation failed"
                );
                // Return generic error to client
                (StatusCode::BAD_REQUEST, "Invalid input data").into_response()
            }
            Self::DatabaseError(e) => {
                // Log detailed database error internally
                tracing::error!(
                    database_error = ?e,
                    "Database operation failed"
                );
                // Return generic error to client
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::SerializationError(e) => {
                // Log detailed serialization error internally
                tracing::error!(
                    serialization_error = ?e,
                    "JSON serialization failed"
                );
                // Return generic error to client
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

// Implement From traits for convenient error conversion
impl From<validator::ValidationErrors> for CheckInError {
    fn from(e: validator::ValidationErrors) -> Self {
        CheckInError::ValidationFailed(e)
    }
}

impl From<rusqlite::Error> for CheckInError {
    fn from(e: rusqlite::Error) -> Self {
        CheckInError::DatabaseError(e)
    }
}

impl From<serde_json::Error> for CheckInError {
    fn from(e: serde_json::Error) -> Self {
        CheckInError::SerializationError(e)
    }
}
