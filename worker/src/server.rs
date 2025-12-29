use crate::config::Config;
use crate::embeddings::EmbeddingsDatabase;
use crate::face::FaceModel;
use crate::handlers;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub async fn start(
    config: Config,
    embeddings_db: Arc<EmbeddingsDatabase>,
    face_model: Arc<FaceModel>,
) -> anyhow::Result<()> {
    let port = config.port;
    let config = Arc::new(config);

    let app = Router::new()
        .route("/authenticate", post(handlers::authenticate_handler))
        .route("/health", get(handlers::health_handler))
        .route("/ready", get(handlers::ready_handler))
        .layer(Extension(config))
        .layer(Extension(embeddings_db))
        .layer(Extension(face_model))
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Worker listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
