use axum::Json;
use serde_json::{json, Value};

pub async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy"
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
    }
}
