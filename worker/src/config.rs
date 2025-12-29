use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub embeddings_path: String,
    pub model_path: String,
    pub match_threshold: f32,
    #[allow(dead_code)]
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            embeddings_path: env::var("EMBEDDINGS_PATH")
                .unwrap_or_else(|_| "/etc/embeddings/data.json".to_string()),
            model_path: env::var("MODEL_PATH")
                .unwrap_or_else(|_| "/models/face_recognition.onnx".to_string()),
            match_threshold: env::var("MATCH_THRESHOLD")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()?,
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_config_from_env_defaults() {
        // Clear env vars
        env::remove_var("PORT");
        env::remove_var("EMBEDDINGS_PATH");
        env::remove_var("MODEL_PATH");
        env::remove_var("MATCH_THRESHOLD");
        env::remove_var("RUST_LOG");

        let config = Config::from_env().unwrap();

        assert_eq!(config.port, 8080);
        assert_eq!(config.embeddings_path, "/etc/embeddings/data.json");
        assert_eq!(config.model_path, "/models/face_recognition.onnx");
        assert_eq!(config.match_threshold, 0.7);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    #[serial]
    fn test_config_from_env_custom() {
        env::set_var("PORT", "9090");
        env::set_var("EMBEDDINGS_PATH", "/tmp/embeddings.json");
        env::set_var("MODEL_PATH", "/tmp/model.onnx");
        env::set_var("MATCH_THRESHOLD", "0.85");
        env::set_var("RUST_LOG", "debug");

        let config = Config::from_env().unwrap();

        assert_eq!(config.port, 9090);
        assert_eq!(config.embeddings_path, "/tmp/embeddings.json");
        assert_eq!(config.model_path, "/tmp/model.onnx");
        assert_eq!(config.match_threshold, 0.85);
        assert_eq!(config.log_level, "debug");

        // Cleanup
        env::remove_var("PORT");
        env::remove_var("EMBEDDINGS_PATH");
        env::remove_var("MODEL_PATH");
        env::remove_var("MATCH_THRESHOLD");
        env::remove_var("RUST_LOG");
    }

    #[test]
    #[serial]
    fn test_config_invalid_port() {
        env::set_var("PORT", "invalid");

        let result = Config::from_env();
        assert!(result.is_err());

        env::remove_var("PORT");
    }

    #[test]
    #[serial]
    fn test_config_invalid_threshold() {
        env::set_var("MATCH_THRESHOLD", "not_a_number");

        let result = Config::from_env();
        assert!(result.is_err());

        env::remove_var("MATCH_THRESHOLD");
    }
}
