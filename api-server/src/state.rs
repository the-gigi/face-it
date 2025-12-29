use crate::kube::{PodManager, PodOperations};
use std::sync::Arc;

/// Application state shared across all handlers
///
/// This wrapper provides a concrete type for Axum's state management,
/// avoiding issues with trait object types in the type system.
#[derive(Clone)]
pub struct AppState {
    pub pod_manager: Arc<PodManager<dyn PodOperations>>,
}

impl AppState {
    pub fn new(pod_manager: Arc<PodManager<dyn PodOperations>>) -> Self {
        Self { pod_manager }
    }
}
