// Re-export commonly used items
pub mod error;
pub mod types;

// Convenience re-exports
pub use error::CommonError;
pub use types::{AuthError, AuthRequest, AuthResponse, EmbeddingsData, UserEmbedding};
