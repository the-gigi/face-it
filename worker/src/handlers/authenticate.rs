use crate::config::Config;
use crate::embeddings::EmbeddingsDatabase;
use crate::error::{WorkerError, WorkerResult};
use crate::face::FaceModel;
use axum::{Extension, Json};
use base64::Engine;
use common::{AuthRequest, AuthResponse};
use std::sync::Arc;
use std::time::Instant;

pub async fn authenticate_handler(
    Extension(config): Extension<Arc<Config>>,
    Extension(embeddings_db): Extension<Arc<EmbeddingsDatabase>>,
    Extension(face_model): Extension<Arc<FaceModel>>,
    Json(req): Json<AuthRequest>,
) -> WorkerResult<Json<AuthResponse>> {
    let start = Instant::now();

    // 1. Decode base64 image
    let image_data = base64::engine::general_purpose::STANDARD
        .decode(&req.image_base64)
        .map_err(|e| WorkerError::InvalidInput(format!("Invalid base64: {}", e)))?;

    // 2. Generate embedding from input image
    let input_embedding = face_model.generate_embedding(&image_data)?;

    // 3. Search for match in embeddings database
    let match_result = embeddings_db.find_match(&input_embedding, config.match_threshold);

    match match_result {
        Some((user_id, confidence)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            // Get user name from database
            let user_name = embeddings_db.get_user_name(&user_id);
            Ok(Json(AuthResponse {
                matched: true,
                user_id: Some(user_id),
                user_name,
                confidence,
                duration_ms: Some(duration_ms),
            }))
        }
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(Json(AuthResponse {
                matched: false,
                user_id: None,
                user_name: None,
                confidence: 0.0,
                duration_ms: Some(duration_ms),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use common::UserEmbedding;

    fn create_test_config() -> Config {
        Config {
            port: 8080,
            embeddings_path: "/tmp/test.json".to_string(),
            model_path: "/tmp/model.onnx".to_string(),
            match_threshold: 0.7,
            log_level: "info".to_string(),
        }
    }

    fn create_test_db() -> EmbeddingsDatabase {
        // Generate a content-based embedding for test image data
        // This matches what the placeholder model will generate
        let test_image_data = b"fake image data";
        let mut embedding = vec![0.0f32; 512];

        let chunk_size = (test_image_data.len() / 512).max(1);
        for (i, chunk) in test_image_data.chunks(chunk_size).enumerate() {
            if i >= 512 {
                break;
            }
            let sum: u32 = chunk.iter().map(|&b| b as u32).sum();
            embedding[i] = ((sum % 2000) as f32 / 1000.0) - 1.0;
        }

        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        let embeddings = vec![UserEmbedding {
            user_id: "user1".to_string(),
            name: "Test User 1".to_string(),
            embedding,
        }];
        EmbeddingsDatabase::new(embeddings)
    }

    #[tokio::test]
    async fn test_authenticate_handler_match_found() {
        let config = Arc::new(create_test_config());
        let embeddings_db = Arc::new(create_test_db());
        let face_model = Arc::new(FaceModel::load("/tmp/model.onnx").unwrap());

        let request = AuthRequest {
            image_base64: base64::engine::general_purpose::STANDARD.encode(b"fake image data"),
        };

        let result = authenticate_handler(
            Extension(config),
            Extension(embeddings_db),
            Extension(face_model),
            Json(request),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.user_id, Some("user1".to_string()));
        assert!(response.confidence >= 0.7);
        assert!(response.duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_authenticate_handler_invalid_base64() {
        let config = Arc::new(create_test_config());
        let embeddings_db = Arc::new(create_test_db());
        let face_model = Arc::new(FaceModel::load("/tmp/model.onnx").unwrap());

        let request = AuthRequest {
            image_base64: "invalid!@#$%".to_string(),
        };

        let result = authenticate_handler(
            Extension(config),
            Extension(embeddings_db),
            Extension(face_model),
            Json(request),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
