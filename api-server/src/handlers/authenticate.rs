use crate::error::{ApiError, ApiResult};
use crate::kube::{PodManager, PodOperations};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use common::{AuthRequest, AuthResponse};
use std::time::Instant;

/// Authenticate handler - proxies requests to worker pods
///
/// Process:
/// 1. Acquire an available worker pod from the pool
/// 2. Forward the authentication request to the worker
/// 3. Release the pod back to the pool
/// 4. Return the response to the client
#[axum::debug_handler]
pub async fn authenticate_handler(
    State(state): State<AppState>,
    Json(req): Json<AuthRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let start = Instant::now();

    // Acquire a worker pod
    let pod = state
        .pod_manager
        .acquire_pod()
        .await?
        .ok_or(ApiError::NoWorkers)?;

    // Get pod IP
    let pod_ip = PodManager::<dyn PodOperations>::get_pod_ip(&pod)?;
    let worker_url = format!("http://{}:8080/authenticate", pod_ip);

    tracing::info!("Forwarding request to worker at {}", worker_url);

    // Forward request to worker with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ApiError::Internal(format!("Failed to build HTTP client: {}", e)))?;

    let response = client
        .post(&worker_url)
        .json(&req)
        .send()
        .await
        .map_err(|e| ApiError::WorkerRequest(format!("Request failed: {}", e)))?;

    // Release pod back to pool (best effort)
    if let Err(e) = state.pod_manager.release_pod(&pod).await {
        tracing::error!("Failed to release pod: {}", e);
        // Continue anyway - pod will eventually timeout
    }

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(ApiError::WorkerRequest(format!(
            "Worker returned {}: {}",
            status, error_text
        )));
    }

    // Parse worker response
    let mut auth_response: AuthResponse = response
        .json()
        .await
        .map_err(|e| ApiError::WorkerRequest(format!("Invalid response: {}", e)))?;

    // Override duration to include API server overhead
    let total_duration = start.elapsed().as_millis() as u64;
    auth_response.duration_ms = Some(total_duration);

    Ok(Json(auth_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::mock::MockPodOperations;
    use crate::kube::PodOperations;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_authenticate_handler_no_workers() {
        let mock_ops: Arc<dyn PodOperations> = Arc::new(MockPodOperations::new());
        let manager = Arc::new(PodManager::new(
            mock_ops,
            "test-ns".to_string(),
            "app=test,status=ready".to_string(),
        ));

        let state = AppState::new(manager);

        let request = AuthRequest {
            image_base64: "fake_image".to_string(),
        };

        let result = authenticate_handler(State(state), Json(request)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::NoWorkers => {}
            e => panic!("Expected NoWorkers error, got {:?}", e),
        }
    }
}
