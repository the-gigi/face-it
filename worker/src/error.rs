use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Image processing error: {0}")]
    Image(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("No match found")]
    #[allow(dead_code)]
    NoMatch,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl IntoResponse for WorkerError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            WorkerError::NoMatch => (
                StatusCode::NOT_FOUND,
                "no_match",
                "No matching face found".to_string(),
            ),
            WorkerError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, "invalid_input", msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                self.to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

pub type WorkerResult<T> = Result<T, WorkerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_error_display() {
        let err = WorkerError::NoMatch;
        assert_eq!(err.to_string(), "No match found");

        let err = WorkerError::InvalidInput("bad data".to_string());
        assert_eq!(err.to_string(), "Invalid input: bad data");

        let err = WorkerError::Image("decode failed".to_string());
        assert_eq!(err.to_string(), "Image processing error: decode failed");
    }

    #[test]
    fn test_worker_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let worker_err: WorkerError = io_err.into();
        assert!(worker_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_worker_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let worker_err: WorkerError = json_err.into();
        assert!(worker_err.to_string().contains("JSON error"));
    }
}
