use crate::error::ApiResult;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;

/// Abstraction for pod operations to enable testing with mocks
#[async_trait]
pub trait PodOperations: Send + Sync {
    /// List pods in a namespace matching a label selector
    async fn list_pods(&self, namespace: &str, label_selector: &str) -> ApiResult<Vec<Pod>>;

    /// Get a specific pod by name in a namespace
    async fn get_pod(&self, namespace: &str, name: &str) -> ApiResult<Pod>;

    /// Patch a pod's labels (for atomic compare-and-swap operations)
    /// Returns the updated pod with new resource version
    async fn patch_pod_labels(
        &self,
        namespace: &str,
        name: &str,
        labels: Vec<(String, String)>,
        resource_version: &str,
    ) -> ApiResult<Pod>;
}
