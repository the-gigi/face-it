use crate::handlers;
use crate::state::AppState;
use axum::{routing::post, Router};
use tower_http::trace::TraceLayer;

/// Build the HTTP server with all routes and middleware
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/authenticate", post(handlers::authenticate_handler))
        .route("/health", axum::routing::get(handlers::health_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::mock::MockPodOperations;
    use crate::kube::{PodManager, PodOperations};
    use axum::body::Body;
    use axum::http::StatusCode;
    use http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let mock_ops: Arc<dyn PodOperations> = Arc::new(MockPodOperations::new());
        let manager = Arc::new(PodManager::new(
            mock_ops,
            "test-ns".to_string(),
            "app=test".to_string(),
        ));

        let state = AppState::new(manager);
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_authenticate_endpoint_no_workers() {
        let mock_ops: Arc<dyn PodOperations> = Arc::new(MockPodOperations::new());
        let manager = Arc::new(PodManager::new(
            mock_ops,
            "test-ns".to_string(),
            "app=test,status=ready".to_string(),
        ));

        let state = AppState::new(manager);
        let app = build_router(state);

        let request_body = r#"{"image_base64":"fake"}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/authenticate")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
