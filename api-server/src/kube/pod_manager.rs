use crate::error::{ApiError, ApiResult};
use crate::kube::traits::PodOperations;
use k8s_openapi::api::core::v1::Pod;
use rand::seq::SliceRandom;
use std::sync::Arc;

/// Pod pool manager with optimistic locking
///
/// Manages a pool of worker pods, using pod labels for state tracking:
/// - status=ready: Pod is available in the pool
/// - status=busy: Pod is currently in use
///
/// Uses Kubernetes resource versions for optimistic locking to handle
/// concurrent access from multiple API server instances.
pub struct PodManager<T: PodOperations + ?Sized> {
    pod_ops: Arc<T>,
    namespace: String,
    selector: String,
    max_retries: u32,
}

impl<T: PodOperations + ?Sized> Clone for PodManager<T> {
    fn clone(&self) -> Self {
        Self {
            pod_ops: Arc::clone(&self.pod_ops),
            namespace: self.namespace.clone(),
            selector: self.selector.clone(),
            max_retries: self.max_retries,
        }
    }
}

impl<T: PodOperations + ?Sized> PodManager<T> {
    pub fn new(pod_ops: Arc<T>, namespace: String, selector: String) -> Self {
        Self {
            pod_ops,
            namespace,
            selector,
            max_retries: 5,
        }
    }

    /// Acquire an available pod from the pool
    ///
    /// Searches for pods with status=ready and attempts to mark one as busy.
    /// Uses optimistic locking via resource versions to handle race conditions.
    /// Returns None if no pods are available after retries.
    pub async fn acquire_pod(&self) -> ApiResult<Option<Pod>> {
        for attempt in 0..self.max_retries {
            // List available pods
            let pods = self
                .pod_ops
                .list_pods(&self.namespace, &self.selector)
                .await?;

            if pods.is_empty() {
                tracing::warn!("No worker pods found with selector: {}", self.selector);
                return Ok(None);
            }

            // Randomly select a pod to avoid thundering herd
            let mut shuffled = pods.clone();
            {
                let mut rng = rand::thread_rng();
                shuffled.shuffle(&mut rng);
                // rng is dropped here, before any await points
            }

            // Try to acquire each pod
            for pod in shuffled {
                let pod_name = pod.metadata.name.as_ref().unwrap();
                let resource_version = pod.metadata.resource_version.as_ref().ok_or_else(|| {
                    ApiError::Internal("Pod missing resource version".to_string())
                })?;

                // Attempt to mark as busy with optimistic locking
                match self
                    .pod_ops
                    .patch_pod_labels(
                        &self.namespace,
                        pod_name,
                        vec![("status".to_string(), "busy".to_string())],
                        resource_version,
                    )
                    .await
                {
                    Ok(updated_pod) => {
                        tracing::info!("Acquired pod {} (attempt {})", pod_name, attempt + 1);
                        return Ok(Some(updated_pod));
                    }
                    Err(ApiError::Kubernetes(ref msg))
                        if msg.contains("version mismatch") || msg.contains("conflict") =>
                    {
                        // Another instance acquired this pod, try next one
                        tracing::debug!(
                            "Pod {} already acquired by another instance, trying next pod",
                            pod_name
                        );
                        continue;
                    }
                    Err(e) => {
                        // Other error, propagate
                        return Err(e);
                    }
                }
            }

            // All pods in this batch were taken, retry
            tracing::debug!("All pods taken in attempt {}, retrying", attempt + 1);
        }

        // No pods available after all retries
        Ok(None)
    }

    /// Release a pod back to the pool
    ///
    /// Marks a pod as ready again after use.
    /// Uses optimistic locking to ensure safe concurrent updates.
    pub async fn release_pod(&self, pod: &Pod) -> ApiResult<()> {
        let pod_name = pod
            .metadata
            .name
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Pod missing name".to_string()))?;

        let resource_version = pod
            .metadata
            .resource_version
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Pod missing resource version".to_string()))?;

        // Mark as ready
        self.pod_ops
            .patch_pod_labels(
                &self.namespace,
                pod_name,
                vec![("status".to_string(), "ready".to_string())],
                resource_version,
            )
            .await?;

        tracing::info!("Released pod {}", pod_name);
        Ok(())
    }

    /// Get pod IP address for making requests
    pub fn get_pod_ip(pod: &Pod) -> ApiResult<String> {
        pod.status
            .as_ref()
            .and_then(|status| status.pod_ip.as_ref())
            .cloned()
            .ok_or_else(|| ApiError::Internal("Pod missing IP address".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::mock::MockPodOperations;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_acquire_pod_success() {
        let mock_ops = Arc::new(MockPodOperations::new());

        // Add a ready pod
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());
        labels.insert("status".to_string(), "ready".to_string());

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", labels);
        mock_ops.add_pod("test-ns", pod);

        // Create manager
        let manager = PodManager::new(
            mock_ops.clone(),
            "test-ns".to_string(),
            "app=test,status=ready".to_string(),
        );

        // Acquire pod
        let acquired = manager.acquire_pod().await.unwrap();
        assert!(acquired.is_some());

        let pod = acquired.unwrap();
        assert_eq!(pod.metadata.name.as_ref().unwrap(), "pod1");
        assert_eq!(
            pod.metadata.labels.as_ref().unwrap().get("status"),
            Some(&"busy".to_string())
        );
    }

    #[tokio::test]
    async fn test_acquire_pod_no_pods() {
        let mock_ops = Arc::new(MockPodOperations::new());

        let manager = PodManager::new(
            mock_ops,
            "test-ns".to_string(),
            "app=test,status=ready".to_string(),
        );

        let acquired = manager.acquire_pod().await.unwrap();
        assert!(acquired.is_none());
    }

    #[tokio::test]
    async fn test_release_pod() {
        let mock_ops = Arc::new(MockPodOperations::new());

        // Add a busy pod
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());
        labels.insert("status".to_string(), "busy".to_string());

        let pod = MockPodOperations::create_test_pod("pod1", "test-ns", labels);
        mock_ops.add_pod("test-ns", pod.clone());

        let manager = PodManager::new(
            mock_ops.clone(),
            "test-ns".to_string(),
            "app=test".to_string(),
        );

        // Release pod
        manager.release_pod(&pod).await.unwrap();

        // Verify it's marked as ready
        let released = mock_ops.get_pod("test-ns", "pod1").await.unwrap();
        assert_eq!(
            released.metadata.labels.as_ref().unwrap().get("status"),
            Some(&"ready".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_pod_ip() {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let mut pod = MockPodOperations::create_test_pod("pod1", "test-ns", labels);

        // Add IP to pod status
        pod.status = Some(k8s_openapi::api::core::v1::PodStatus {
            pod_ip: Some("10.0.0.1".to_string()),
            ..Default::default()
        });

        let ip = PodManager::<MockPodOperations>::get_pod_ip(&pod).unwrap();
        assert_eq!(ip, "10.0.0.1");
    }

    #[tokio::test]
    async fn test_acquire_pod_concurrent_conflict() {
        let mock_ops = Arc::new(MockPodOperations::new());

        // Add two ready pods
        for i in 1..=2 {
            let mut labels = HashMap::new();
            labels.insert("app".to_string(), "test".to_string());
            labels.insert("status".to_string(), "ready".to_string());

            let pod = MockPodOperations::create_test_pod(&format!("pod{}", i), "test-ns", labels);
            mock_ops.add_pod("test-ns", pod);
        }

        let manager = PodManager::new(
            mock_ops,
            "test-ns".to_string(),
            "app=test,status=ready".to_string(),
        );

        // Acquire both pods
        let pod1 = manager.acquire_pod().await.unwrap();
        let pod2 = manager.acquire_pod().await.unwrap();

        assert!(pod1.is_some());
        assert!(pod2.is_some());

        // Third attempt should fail (no more pods)
        let pod3 = manager.acquire_pod().await.unwrap();
        assert!(pod3.is_none());
    }
}
