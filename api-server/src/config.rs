use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub worker_namespace: String,
    pub worker_selector: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            worker_namespace: env::var("WORKER_NAMESPACE")
                .unwrap_or_else(|_| "face-it-workers".to_string()),
            worker_selector: env::var("WORKER_SELECTOR")
                .unwrap_or_else(|_| "app=face-recognition-worker,status=ready".to_string()),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_from_env_defaults() {
        // Clear env vars
        env::remove_var("PORT");
        env::remove_var("WORKER_NAMESPACE");
        env::remove_var("WORKER_SELECTOR");
        env::remove_var("RUST_LOG");

        let config = Config::from_env().unwrap();

        assert_eq!(config.port, 8080);
        assert_eq!(config.worker_namespace, "face-it-workers");
        assert_eq!(
            config.worker_selector,
            "app=face-recognition-worker,status=ready"
        );
        assert_eq!(config.log_level, "info");
    }

    #[test]
    #[serial]
    fn test_config_from_env_custom() {
        env::set_var("PORT", "9090");
        env::set_var("WORKER_NAMESPACE", "custom-namespace");
        env::set_var("WORKER_SELECTOR", "app=custom,status=active");
        env::set_var("RUST_LOG", "debug");

        let config = Config::from_env().unwrap();

        assert_eq!(config.port, 9090);
        assert_eq!(config.worker_namespace, "custom-namespace");
        assert_eq!(config.worker_selector, "app=custom,status=active");
        assert_eq!(config.log_level, "debug");

        // Cleanup
        env::remove_var("PORT");
        env::remove_var("WORKER_NAMESPACE");
        env::remove_var("WORKER_SELECTOR");
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
}
