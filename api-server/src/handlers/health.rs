use axum::Json;
use serde_json::{json, Value};

/// Health check endpoint
///
/// Returns 200 OK with status information
pub async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "face-it-api-server"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await;
        let value = response.0;

        assert_eq!(value["status"], "healthy");
        assert_eq!(value["service"], "face-it-api-server");
    }
}
