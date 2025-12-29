mod config;
mod embeddings;
mod error;
mod face;
mod handlers;
mod server;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting worker");

    // Load configuration
    let config = config::Config::from_env()?;

    // Load embeddings database
    tracing::info!("Loading embeddings from {}", config.embeddings_path);
    let embeddings_db =
        embeddings::EmbeddingsDatabase::load_from_file(&config.embeddings_path).await?;
    tracing::info!("Loaded {} embeddings", embeddings_db.count());

    // Load face recognition model
    tracing::info!("Loading face recognition model from {}", config.model_path);
    let face_model = face::FaceModel::load(&config.model_path)?;
    tracing::info!("Model loaded successfully");

    // Start HTTP server
    server::start(config, Arc::new(embeddings_db), Arc::new(face_model)).await?;

    Ok(())
}
