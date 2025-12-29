use api_server::{kube, server, Config};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(config.log_level.clone())
        .init();

    tracing::info!("API Server starting");
    tracing::info!("Port: {}", config.port);
    tracing::info!("Worker namespace: {}", config.worker_namespace);
    tracing::info!("Worker selector: {}", config.worker_selector);

    // Initialize Kubernetes client
    tracing::info!("Connecting to Kubernetes...");
    let kube_client = kube::KubeClient::new().await?;
    tracing::info!("Connected to Kubernetes");

    // Create pod manager with dynamic dispatch
    let pod_manager = Arc::new(kube::PodManager::new(
        Arc::new(kube_client) as Arc<dyn kube::PodOperations>,
        config.worker_namespace.clone(),
        config.worker_selector.clone(),
    ));

    // Create application state
    let state = api_server::AppState::new(pod_manager);

    // Build HTTP server
    let app = server::build_router(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("API Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
