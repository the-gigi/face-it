// Module declarations for Kubernetes abstractions
pub mod client;
pub mod mock;
pub mod pod_manager;
pub mod traits;

// Re-exports for convenience
pub use client::KubeClient;
pub use mock::MockPodOperations;
pub use pod_manager::PodManager;
pub use traits::PodOperations;
