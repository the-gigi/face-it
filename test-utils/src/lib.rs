use anyhow::{Context, Result};
/// Test utilities for integration tests
/// Manages kind cluster lifecycle and test fixtures
use std::process::Command;

pub const CLUSTER_NAME: &str = "face-it";
pub const API_NAMESPACE: &str = "face-it-api";
pub const WORKER_NAMESPACE: &str = "face-it-workers";

/// Test fixture that manages kind cluster lifecycle
pub struct KindCluster {
    cluster_name: String,
}

impl KindCluster {
    /// Get or create the test cluster
    /// Idempotent - safe to call multiple times
    pub fn setup() -> Result<Self> {
        let cluster = Self {
            cluster_name: CLUSTER_NAME.to_string(),
        };

        if !cluster.exists()? {
            println!("Creating kind cluster: {}", CLUSTER_NAME);
            cluster.create()?;
        } else {
            println!("Using existing kind cluster: {}", CLUSTER_NAME);
        }

        // Ensure namespaces exist and are clean
        cluster.setup_namespaces()?;

        Ok(cluster)
    }

    /// Check if cluster exists
    fn exists(&self) -> Result<bool> {
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .context("Failed to execute 'kind get clusters'")?;

        if !output.status.success() {
            return Ok(false);
        }

        let clusters = String::from_utf8_lossy(&output.stdout);
        Ok(clusters
            .lines()
            .any(|line| line.trim() == self.cluster_name))
    }

    /// Create new kind cluster with worker nodes
    fn create(&self) -> Result<()> {
        let config = r#"
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
- role: worker
  kubeadmConfigPatches:
  - |
    kind: JoinConfiguration
    nodeRegistration:
      kubeletExtraArgs:
        node-labels: "workload=sensitive"
"#;

        let mut child = Command::new("kind")
            .args([
                "create",
                "cluster",
                "--name",
                &self.cluster_name,
                "--config",
                "-",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .context("Failed to spawn 'kind create cluster'")?;

        // Write config to stdin
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(config.as_bytes())
                .context("Failed to write config to stdin")?;
        }

        // Wait for command to complete
        let status = child.wait().context("Failed to wait for kind create")?;

        if !status.success() {
            anyhow::bail!("kind create cluster failed");
        }

        // Wait for cluster to be ready
        self.wait_for_ready()?;

        Ok(())
    }

    /// Wait for cluster nodes to be ready
    fn wait_for_ready(&self) -> Result<()> {
        println!("Waiting for cluster nodes to be ready...");

        let status = Command::new("kubectl")
            .args([
                "wait",
                "--for=condition=Ready",
                "nodes",
                "--all",
                "--timeout=60s",
            ])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to wait for nodes")?;

        if !status.success() {
            anyhow::bail!("Nodes did not become ready in time");
        }

        Ok(())
    }

    /// Setup and clean namespaces
    fn setup_namespaces(&self) -> Result<()> {
        // Delete namespaces if they exist (clean slate)
        for ns in [API_NAMESPACE, WORKER_NAMESPACE] {
            let _ = self.delete_namespace(ns); // Ignore errors if doesn't exist
        }

        // Create namespaces
        for ns in [API_NAMESPACE, WORKER_NAMESPACE] {
            self.create_namespace(ns)?;
        }

        Ok(())
    }

    /// Create namespace
    fn create_namespace(&self, name: &str) -> Result<()> {
        println!("Creating namespace: {}", name);

        let status = Command::new("kubectl")
            .args(["create", "namespace", name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("Failed to create namespace")?;

        if !status.success() {
            anyhow::bail!("Failed to create namespace: {}", name);
        }

        Ok(())
    }

    /// Delete namespace (for cleanup)
    fn delete_namespace(&self, name: &str) -> Result<()> {
        let status = Command::new("kubectl")
            .args(["delete", "namespace", name, "--ignore-not-found=true"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("Failed to delete namespace")?;

        if !status.success() {
            anyhow::bail!("Failed to delete namespace: {}", name);
        }

        // Wait for namespace to be deleted
        let _ = Command::new("kubectl")
            .args([
                "wait",
                "--for=delete",
                &format!("namespace/{}", name),
                "--timeout=30s",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        Ok(())
    }

    /// Apply RBAC configuration
    pub fn apply_rbac(&self) -> Result<()> {
        println!("Applying RBAC configuration...");

        let rbac_yaml = format!(
            r#"
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: api-server-sa
  namespace: {api_ns}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: pod-manager-role
  namespace: {worker_ns}
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list", "patch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: api-server-pod-manager-binding
  namespace: {worker_ns}
subjects:
- kind: ServiceAccount
  name: api-server-sa
  namespace: {api_ns}
roleRef:
  kind: Role
  name: pod-manager-role
  apiGroup: rbac.authorization.k8s.io
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: worker-sa
  namespace: {worker_ns}
"#,
            api_ns = API_NAMESPACE,
            worker_ns = WORKER_NAMESPACE
        );

        let status = Command::new("kubectl")
            .args(["apply", "-f", "-"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(rbac_yaml.as_bytes())?;
                child.wait()
            })
            .context("Failed to apply RBAC")?;

        if !status.success() {
            anyhow::bail!("Failed to apply RBAC configuration");
        }

        Ok(())
    }

    /// Get cluster name for kubectl context
    pub fn context_name(&self) -> String {
        format!("kind-{}", self.cluster_name)
    }
}

/// Delete the test cluster
/// Call this explicitly if you want to clean up
#[allow(dead_code)]
pub fn teardown_cluster() -> Result<()> {
    println!("Deleting kind cluster: {}", CLUSTER_NAME);

    let status = Command::new("kind")
        .args(["delete", "cluster", "--name", CLUSTER_NAME])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to delete cluster")?;

    if !status.success() {
        anyhow::bail!("Failed to delete cluster");
    }

    Ok(())
}

/// Helper to create a test pod in the cluster
pub async fn create_test_pod(
    namespace: &str,
    name: &str,
    labels: std::collections::HashMap<String, String>,
) -> Result<()> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    let pod = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "labels": labels,
        },
        "spec": {
            "containers": [{
                "name": "test-container",
                "image": "nginx:alpine",
                "ports": [{"containerPort": 8080}],
                "readinessProbe": {
                    "httpGet": {
                        "path": "/",
                        "port": 80,
                    },
                    "initialDelaySeconds": 1,
                    "periodSeconds": 1,
                },
            }],
        },
    });

    let pp = kube::api::PostParams::default();
    pods.create(&pp, &serde_json::from_value(pod)?)
        .await
        .context("Failed to create test pod")?;

    Ok(())
}

/// Helper to delete a test pod
pub async fn delete_test_pod(namespace: &str, name: &str) -> Result<()> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    let dp = kube::api::DeleteParams::default();
    pods.delete(name, &dp)
        .await
        .context("Failed to delete pod")?;

    Ok(())
}

/// Helper to wait for pod to be ready
pub async fn wait_for_pod_ready(namespace: &str, name: &str) -> Result<()> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};
    use std::time::Duration;
    use tokio::time::sleep;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    for _ in 0..30 {
        let pod = pods.get(name).await?;

        if let Some(status) = &pod.status {
            if let Some(conditions) = &status.conditions {
                if conditions
                    .iter()
                    .any(|c| c.type_ == "Ready" && c.status == "True")
                {
                    return Ok(());
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    anyhow::bail!("Pod {} did not become ready in time", name)
}
