use crate::error::{ApiError, ApiResult};
use crate::kube::traits::PodOperations;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// Mock implementation of PodOperations for unit testing
#[derive(Clone)]
pub struct MockPodOperations {
    pods: Arc<Mutex<HashMap<String, Pod>>>,
}

impl MockPodOperations {
    /// Create a new mock with no pods
    pub fn new() -> Self {
        Self {
            pods: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a pod to the mock store
    pub fn add_pod(&self, namespace: &str, pod: Pod) {
        let key = self.make_key(namespace, pod.metadata.name.as_ref().unwrap());
        self.pods.lock().unwrap().insert(key, pod);
    }

    /// Helper to create a test pod
    pub fn create_test_pod(name: &str, namespace: &str, labels: HashMap<String, String>) -> Pod {
        // Convert HashMap to BTreeMap for Pod metadata
        let btree_labels: BTreeMap<String, String> = labels.into_iter().collect();

        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(btree_labels),
                resource_version: Some("1".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_key(&self, namespace: &str, name: &str) -> String {
        format!("{}/{}", namespace, name)
    }

    fn matches_selector(labels: &BTreeMap<String, String>, selector: &str) -> bool {
        // Simple label selector parsing: "key1=value1,key2=value2"
        if selector.is_empty() {
            return true;
        }

        for pair in selector.split(',') {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            if labels.get(key) != Some(&value.to_string()) {
                return false;
            }
        }

        true
    }
}

impl Default for MockPodOperations {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PodOperations for MockPodOperations {
    async fn list_pods(&self, namespace: &str, label_selector: &str) -> ApiResult<Vec<Pod>> {
        let pods = self.pods.lock().unwrap();

        let matching_pods: Vec<Pod> = pods
            .iter()
            .filter(|(key, pod)| {
                key.starts_with(&format!("{}/", namespace))
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .is_some_and(|labels| Self::matches_selector(labels, label_selector))
            })
            .map(|(_, pod)| pod.clone())
            .collect();

        Ok(matching_pods)
    }

    async fn get_pod(&self, namespace: &str, name: &str) -> ApiResult<Pod> {
        let key = self.make_key(namespace, name);
        let pods = self.pods.lock().unwrap();

        pods.get(&key)
            .cloned()
            .ok_or_else(|| ApiError::Kubernetes(format!("Pod not found: {}", key)))
    }

    async fn patch_pod_labels(
        &self,
        namespace: &str,
        name: &str,
        labels: Vec<(String, String)>,
        resource_version: &str,
    ) -> ApiResult<Pod> {
        let key = self.make_key(namespace, name);
        let mut pods = self.pods.lock().unwrap();

        let pod = pods
            .get(&key)
            .ok_or_else(|| ApiError::Kubernetes(format!("Pod not found: {}", key)))?;

        // Check resource version for optimistic locking
        let current_version = pod
            .metadata
            .resource_version
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Pod missing resource version".to_string()))?;

        if current_version != resource_version {
            return Err(ApiError::Kubernetes(format!(
                "Resource version mismatch: expected {}, got {}",
                resource_version, current_version
            )));
        }

        // Create updated pod with new labels and incremented resource version
        let mut updated_pod = pod.clone();
        let mut pod_labels: BTreeMap<String, String> =
            updated_pod.metadata.labels.unwrap_or_default();

        for (key, value) in labels {
            pod_labels.insert(key, value);
        }

        updated_pod.metadata.labels = Some(pod_labels);

        // Increment resource version
        let new_version = current_version
            .parse::<u64>()
            .unwrap_or(0)
            .wrapping_add(1)
            .to_string();
        updated_pod.metadata.resource_version = Some(new_version);

        // Update in store
        pods.insert(key, updated_pod.clone());

        Ok(updated_pod)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_list_pods_empty() {
        let mock = MockPodOperations::new();
        let pods = mock.list_pods("test-ns", "").await.unwrap();
        assert_eq!(pods.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_add_and_list_pods() {
        let mock = MockPodOperations::new();

        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", labels);
        mock.add_pod("test-ns", pod);

        let pods = mock.list_pods("test-ns", "app=test").await.unwrap();
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].metadata.name.as_ref().unwrap(), "pod1");
    }

    #[tokio::test]
    async fn test_mock_list_pods_with_selector() {
        let mock = MockPodOperations::new();

        let mut labels1 = HashMap::new();
        labels1.insert("app".to_string(), "test".to_string());
        labels1.insert("status".to_string(), "ready".to_string());

        let mut labels2 = HashMap::new();
        labels2.insert("app".to_string(), "test".to_string());
        labels2.insert("status".to_string(), "busy".to_string());

        mock.add_pod(
            "test-ns",
            MockPodOperations::create_test_pod("pod1", "test-ns", labels1),
        );
        mock.add_pod(
            "test-ns",
            MockPodOperations::create_test_pod("pod2", "test-ns", labels2),
        );

        let pods = mock
            .list_pods("test-ns", "app=test,status=ready")
            .await
            .unwrap();
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].metadata.name.as_ref().unwrap(), "pod1");
    }

    #[tokio::test]
    async fn test_mock_get_pod() {
        let mock = MockPodOperations::new();

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", HashMap::new());
        mock.add_pod("test-ns", pod);

        let fetched = mock.get_pod("test-ns", "pod1").await.unwrap();
        assert_eq!(fetched.metadata.name.as_ref().unwrap(), "pod1");
    }

    #[tokio::test]
    async fn test_mock_get_pod_not_found() {
        let mock = MockPodOperations::new();
        let result = mock.get_pod("test-ns", "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_patch_pod_labels() {
        let mock = MockPodOperations::new();

        let mut labels = HashMap::new();
        labels.insert("status".to_string(), "ready".to_string());

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", labels);
        mock.add_pod("test-ns", pod);

        let updated = mock
            .patch_pod_labels(
                "test-ns",
                "pod1",
                vec![("status".to_string(), "busy".to_string())],
                "1",
            )
            .await
            .unwrap();

        assert_eq!(
            updated.metadata.labels.as_ref().unwrap().get("status"),
            Some(&"busy".to_string())
        );
        assert_eq!(updated.metadata.resource_version.as_ref().unwrap(), "2");
    }

    #[tokio::test]
    async fn test_mock_patch_pod_version_mismatch() {
        let mock = MockPodOperations::new();

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", HashMap::new());
        mock.add_pod("test-ns", pod);

        let result = mock
            .patch_pod_labels(
                "test-ns",
                "pod1",
                vec![("status".to_string(), "busy".to_string())],
                "999", // Wrong version
            )
            .await;

        assert!(result.is_err());
    }
}
