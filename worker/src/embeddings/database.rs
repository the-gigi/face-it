use crate::error::WorkerResult;
use crate::face::matcher::cosine_similarity;
use common::{EmbeddingsData, UserEmbedding};

pub struct EmbeddingsDatabase {
    embeddings: Vec<UserEmbedding>,
}

impl EmbeddingsDatabase {
    /// Create a new EmbeddingsDatabase from a vector of embeddings
    #[cfg(test)]
    pub fn new(embeddings: Vec<UserEmbedding>) -> Self {
        Self { embeddings }
    }

    pub async fn load_from_file(path: &str) -> WorkerResult<Self> {
        let contents = tokio::fs::read_to_string(path).await?;
        let data: EmbeddingsData = serde_json::from_str(&contents)?;

        Ok(Self {
            embeddings: data.embeddings,
        })
    }

    pub fn count(&self) -> usize {
        self.embeddings.len()
    }

    pub fn find_match(&self, input_embedding: &[f32], threshold: f32) -> Option<(String, f32)> {
        self.embeddings
            .iter()
            .filter_map(|user| {
                let similarity = cosine_similarity(input_embedding, &user.embedding);
                if similarity >= threshold {
                    Some((user.user_id.clone(), similarity))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    pub fn get_user_name(&self, user_id: &str) -> Option<String> {
        self.embeddings
            .iter()
            .find(|user| user.user_id == user_id)
            .map(|user| user.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_load_from_file() {
        // Create temporary embeddings file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = serde_json::json!({
            "embeddings": [
                {
                    "user_id": "user1",
                    "name": "Test User",
                    "embedding": [0.1, 0.2, 0.3]
                }
            ]
        });
        temp_file.write_all(data.to_string().as_bytes()).unwrap();

        // Load embeddings
        let db = EmbeddingsDatabase::load_from_file(temp_file.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(db.count(), 1);
    }

    #[tokio::test]
    async fn test_load_from_file_not_found() {
        let result = EmbeddingsDatabase::load_from_file("/nonexistent/file.json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_from_file_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid json").unwrap();

        let result = EmbeddingsDatabase::load_from_file(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_find_match_above_threshold() {
        let embeddings = vec![
            UserEmbedding {
                user_id: "user1".to_string(),
                name: "User 1".to_string(),
                embedding: vec![1.0, 0.0, 0.0],
            },
            UserEmbedding {
                user_id: "user2".to_string(),
                name: "User 2".to_string(),
                embedding: vec![0.0, 1.0, 0.0],
            },
        ];

        let db = EmbeddingsDatabase { embeddings };

        // Query close to user1
        let query = vec![0.9, 0.1, 0.0];
        let result = db.find_match(&query, 0.8);

        assert!(result.is_some());
        let (user_id, confidence) = result.unwrap();
        assert_eq!(user_id, "user1");
        assert!(confidence > 0.8);
    }

    #[test]
    fn test_find_match_below_threshold() {
        let embeddings = vec![UserEmbedding {
            user_id: "user1".to_string(),
            name: "User 1".to_string(),
            embedding: vec![1.0, 0.0, 0.0],
        }];

        let db = EmbeddingsDatabase { embeddings };

        // Query far from user1
        let query = vec![0.0, 1.0, 0.0];
        let result = db.find_match(&query, 0.8);

        assert!(result.is_none());
    }

    #[test]
    fn test_find_best_match_multiple_candidates() {
        let embeddings = vec![
            UserEmbedding {
                user_id: "user1".to_string(),
                name: "User 1".to_string(),
                embedding: vec![1.0, 0.0, 0.0],
            },
            UserEmbedding {
                user_id: "user2".to_string(),
                name: "User 2".to_string(),
                embedding: vec![0.9, 0.1, 0.0],
            },
        ];

        let db = EmbeddingsDatabase { embeddings };

        // Query that matches both, but closer to user1
        let query = vec![1.0, 0.0, 0.0];
        let result = db.find_match(&query, 0.5);

        assert!(result.is_some());
        let (user_id, _) = result.unwrap();
        assert_eq!(user_id, "user1"); // Should pick best match
    }

    #[test]
    fn test_empty_database() {
        let db = EmbeddingsDatabase { embeddings: vec![] };

        let query = vec![1.0, 0.0, 0.0];
        let result = db.find_match(&query, 0.5);

        assert!(result.is_none());
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_threshold_boundary() {
        let embeddings = vec![UserEmbedding {
            user_id: "user1".to_string(),
            name: "User 1".to_string(),
            embedding: vec![1.0, 0.0],
        }];

        let db = EmbeddingsDatabase { embeddings };

        // Query that gives exactly threshold similarity
        let query = vec![0.707, 0.707]; // 45 degrees, similarity ≈ 0.707

        // Should match at threshold 0.7
        let result = db.find_match(&query, 0.7);
        assert!(result.is_some());

        // Should not match above threshold
        let result = db.find_match(&query, 0.75);
        assert!(result.is_none());
    }
}
