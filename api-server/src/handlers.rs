// Module declarations for HTTP handlers
pub mod authenticate;
pub mod health;

// Re-exports
pub use authenticate::authenticate_handler;
pub use health::health_handler;
