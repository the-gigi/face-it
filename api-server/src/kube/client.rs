use crate::error::{ApiError, ApiResult};
use crate::kube::traits::PodOperations;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{ListParams, Patch, PatchParams},
    Api, Client,
};
use serde_json::json;

/// Real Kubernetes client implementation using kube-rs
pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    /// Create a new Kubernetes client using the default configuration
    /// (in-cluster config or ~/.kube/config)
    pub async fn new() -> ApiResult<Self> {
        let client = Client::try_default()
            .await
            .map_err(|e| ApiError::Kubernetes(format!("Failed to create K8s client: {}", e)))?;

        Ok(Self { client })
    }

    /// Create a Kubernetes client from an explicit kube::Client
    pub fn from_client(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PodOperations for KubeClient {
    async fn list_pods(&self, namespace: &str, label_selector: &str) -> ApiResult<Vec<Pod>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);

        let list_params = ListParams::default().labels(label_selector);

        let pod_list = pods.list(&list_params).await?;

        Ok(pod_list.items)
    }

    async fn get_pod(&self, namespace: &str, name: &str) -> ApiResult<Pod> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pod = pods.get(name).await?;
        Ok(pod)
    }

    async fn patch_pod_labels(
        &self,
        namespace: &str,
        name: &str,
        labels: Vec<(String, String)>,
        resource_version: &str,
    ) -> ApiResult<Pod> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);

        // Build labels object
        let mut labels_obj = serde_json::Map::new();
        for (key, value) in labels {
            labels_obj.insert(key, json!(value));
        }

        // JSON patch to update labels with resource version check
        let patch = json!({
            "metadata": {
                "labels": labels_obj,
                "resourceVersion": resource_version
            }
        });

        // Use strategic merge patch with optimistic locking
        let patch_params = PatchParams::default();
        let patched_pod = pods
            .patch(name, &patch_params, &Patch::Strategic(&patch))
            .await?;

        Ok(patched_pod)
    }
}

#[cfg(test)]
mod tests {
    // Note: KubeClient tests require a real Kubernetes cluster
    // We test this in integration tests with kind
    // Unit tests focus on the mock implementation in mock.rs
}
