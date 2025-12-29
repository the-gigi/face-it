use serde::{Deserialize, Serialize};

/// Request to authenticate a face
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub image_base64: String,
}

/// Successful authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Error response for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Internal representation of a user embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmbedding {
    pub user_id: String,
    pub name: String,
    pub embedding: Vec<f32>,
}

/// Database of embeddings (loaded from Secret)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsData {
    pub embeddings: Vec<UserEmbedding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_request_serialization() {
        let req = AuthRequest {
            image_base64: "base64data".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("base64data"));
        assert!(json.contains("image_base64"));
    }

    #[test]
    fn test_auth_response_serialization() {
        let resp = AuthResponse {
            matched: true,
            user_id: Some("user123".to_string()),
            user_name: Some("Test User".to_string()),
            confidence: 0.95,
            duration_ms: Some(42),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("0.95"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_auth_response_skip_none_duration() {
        let resp = AuthResponse {
            matched: true,
            user_id: Some("user123".to_string()),
            user_name: Some("Test User".to_string()),
            confidence: 0.95,
            duration_ms: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("duration_ms"));
    }

    #[test]
    fn test_user_embedding_serialization() {
        let embedding = UserEmbedding {
            user_id: "user1".to_string(),
            name: "John Doe".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
        };

        let json = serde_json::to_string(&embedding).unwrap();
        let deserialized: UserEmbedding = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, "user1");
        assert_eq!(deserialized.name, "John Doe");
        assert_eq!(deserialized.embedding.len(), 3);
    }

    #[test]
    fn test_embeddings_data_serialization() {
        let data = EmbeddingsData {
            embeddings: vec![
                UserEmbedding {
                    user_id: "user1".to_string(),
                    name: "User 1".to_string(),
                    embedding: vec![0.1, 0.2],
                },
                UserEmbedding {
                    user_id: "user2".to_string(),
                    name: "User 2".to_string(),
                    embedding: vec![0.3, 0.4],
                },
            ],
        };

        let json = serde_json::to_string(&data).unwrap();
        let deserialized: EmbeddingsData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.embeddings.len(), 2);
        assert_eq!(deserialized.embeddings[0].user_id, "user1");
        assert_eq!(deserialized.embeddings[1].user_id, "user2");
    }
}
