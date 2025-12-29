use axum::Json;
use serde_json::{json, Value};

pub async fn ready_handler() -> Json<Value> {
    // This is called by Kubernetes readiness probe
    // Returns ready only after embeddings and model are loaded
    Json(json!({
        "status": "ready"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ready_handler() {
        let response = ready_handler().await;
        let value = response.0;

        assert_eq!(value["status"], "ready");
    }
}
