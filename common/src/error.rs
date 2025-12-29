use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());

        let common_err: CommonError = json_err.unwrap_err().into();
        assert!(common_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_invalid_data_error() {
        let err = CommonError::InvalidData("test error".to_string());
        assert_eq!(err.to_string(), "Invalid data: test error");
    }
}
