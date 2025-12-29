use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("No available worker pods")]
    NoWorkers,

    #[error("Worker request failed: {0}")]
    WorkerRequest(String),

    #[error("Kubernetes error: {0}")]
    Kubernetes(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::NoWorkers => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            ApiError::WorkerRequest(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ApiError::Kubernetes(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl From<kube::Error> for ApiError {
    fn from(err: kube::Error) -> Self {
        ApiError::Kubernetes(err.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::WorkerRequest(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_no_workers() {
        let err = ApiError::NoWorkers;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_error_worker_request() {
        let err = ApiError::WorkerRequest("timeout".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_error_kubernetes() {
        let err = ApiError::Kubernetes("connection failed".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_invalid_input() {
        let err = ApiError::InvalidInput("missing field".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_internal() {
        let err = ApiError::Internal("unexpected state".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
