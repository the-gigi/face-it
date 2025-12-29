use std::collections::HashMap;
/// Integration tests for pod pool management
/// These tests run against a real kind cluster
///
/// Run with: cargo test --test integration_pod_pool -- --ignored --test-threads=1
use test_utils::{
    create_test_pod, delete_test_pod, wait_for_pod_ready, KindCluster, WORKER_NAMESPACE,
};

/// Setup function that runs before each test
fn setup() -> KindCluster {
    // This is idempotent - safe to call for every test
    KindCluster::setup().expect("Failed to setup kind cluster")
}

#[tokio::test]
#[ignore] // Run explicitly with --ignored flag
async fn test_cluster_exists() {
    let cluster = setup();
    println!("✓ Cluster ready: {}", cluster.context_name());
}

#[tokio::test]
#[ignore]
async fn test_create_and_list_pods() -> Result<(), Box<dyn std::error::Error>> {
    let _cluster = setup();

    // Create test pods
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "worker".to_string());
    labels.insert("pool".to_string(), "idle".to_string());

    create_test_pod(WORKER_NAMESPACE, "test-worker-1", labels.clone()).await?;
    create_test_pod(WORKER_NAMESPACE, "test-worker-2", labels.clone()).await?;

    // Wait for pods to be ready
    wait_for_pod_ready(WORKER_NAMESPACE, "test-worker-1").await?;
    wait_for_pod_ready(WORKER_NAMESPACE, "test-worker-2").await?;

    // List pods using kube client
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, WORKER_NAMESPACE);

    let pod_list = pods.list(&Default::default()).await?;
    assert_eq!(pod_list.items.len(), 2);

    // Cleanup
    delete_test_pod(WORKER_NAMESPACE, "test-worker-1").await?;
    delete_test_pod(WORKER_NAMESPACE, "test-worker-2").await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_pod_label_patching() -> Result<(), Box<dyn std::error::Error>> {
    let _cluster = setup();

    // Create test pod
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "worker".to_string());
    labels.insert("pool".to_string(), "idle".to_string());

    create_test_pod(WORKER_NAMESPACE, "patch-test-pod", labels).await?;
    wait_for_pod_ready(WORKER_NAMESPACE, "patch-test-pod").await?;

    // Patch pod label
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::{Patch, PatchParams};
    use kube::{Api, Client};
    use serde_json::json;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, WORKER_NAMESPACE);

    let patch = json!({
        "metadata": {
            "labels": {
                "pool": "working"
            }
        }
    });

    let pp = PatchParams::default();
    pods.patch("patch-test-pod", &pp, &Patch::Merge(patch))
        .await?;

    // Verify label changed
    let pod = pods.get("patch-test-pod").await?;
    let labels = pod.metadata.labels.unwrap();
    assert_eq!(labels.get("pool"), Some(&"working".to_string()));

    // Cleanup
    delete_test_pod(WORKER_NAMESPACE, "patch-test-pod").await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_optimistic_locking_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let _cluster = setup();

    // Create test pod
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "worker".to_string());
    labels.insert("pool".to_string(), "idle".to_string());

    create_test_pod(WORKER_NAMESPACE, "lock-test-pod", labels).await?;
    wait_for_pod_ready(WORKER_NAMESPACE, "lock-test-pod").await?;

    use k8s_openapi::api::core::v1::Pod;
    use kube::api::{Patch, PatchParams};
    use kube::{Api, Client};
    use serde_json::json;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, WORKER_NAMESPACE);

    // Get initial resource version
    let pod = pods.get("lock-test-pod").await?;
    let old_rv = pod.metadata.resource_version.clone().unwrap();

    // Patch 1: Update to "working" (should succeed)
    let patch1 = json!({
        "metadata": {
            "labels": {"pool": "working"},
            "resourceVersion": old_rv
        }
    });

    let pp = PatchParams::default();
    pods.patch("lock-test-pod", &pp, &Patch::Merge(patch1))
        .await?;

    // Patch 2: Try to update using OLD resource version (should fail)
    let patch2 = json!({
        "metadata": {
            "labels": {"pool": "busy"},
            "resourceVersion": old_rv  // Stale!
        }
    });

    let result = pods
        .patch("lock-test-pod", &pp, &Patch::Merge(patch2))
        .await;

    // This should fail with conflict
    assert!(result.is_err(), "Expected conflict error");

    // Cleanup
    delete_test_pod(WORKER_NAMESPACE, "lock-test-pod").await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_concurrent_pod_detachment() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = setup();
    cluster.apply_rbac()?;

    // Create 5 test pods
    for i in 1..=5 {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "worker".to_string());
        labels.insert("pool".to_string(), "idle".to_string());

        let name = format!("concurrent-worker-{}", i);
        create_test_pod(WORKER_NAMESPACE, &name, labels).await?;
    }

    // Wait for all pods to be ready
    for i in 1..=5 {
        let name = format!("concurrent-worker-{}", i);
        wait_for_pod_ready(WORKER_NAMESPACE, &name).await?;
    }

    // TODO: Test concurrent detachment with real PodManager
    // This requires implementing the trait-based abstraction first
    // For now, just verify pods exist

    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, WORKER_NAMESPACE);

    let pod_list = pods.list(&Default::default()).await?;
    assert_eq!(pod_list.items.len(), 5);

    // Cleanup
    for i in 1..=5 {
        let name = format!("concurrent-worker-{}", i);
        delete_test_pod(WORKER_NAMESPACE, &name).await?;
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_deployment_detachment() -> Result<(), Box<dyn std::error::Error>> {
    let _cluster = setup();

    println!("\n=== Testing Deployment Pod Detachment ===\n");

    // Apply RBAC first
    println!("1. Applying RBAC configuration...");

    use std::process::Command;
    let output = Command::new("kubectl")
        .args([
            "apply",
            "-f",
            "../k8s/rbac.yaml",
            "--context",
            "kind-face-it",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Failed to apply RBAC: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to apply RBAC".into());
    }
    println!("✓ RBAC applied");

    // Apply embeddings secret
    println!("\n2. Applying embeddings secret...");
    let output = Command::new("kubectl")
        .args([
            "apply",
            "-f",
            "../k8s/embeddings-secret.yaml",
            "--context",
            "kind-face-it",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Failed to apply secret: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to apply secret".into());
    }
    println!("✓ Secret applied");

    // Apply worker deployment
    println!("\n3. Applying worker deployment...");
    let output = Command::new("kubectl")
        .args([
            "apply",
            "-f",
            "../k8s/worker-deployment.yaml",
            "--context",
            "kind-face-it",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Failed to apply deployment: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to apply deployment".into());
    }
    println!("✓ Deployment applied");

    // Wait for pods to be ready
    println!("\n2. Waiting for pods to be ready...");

    use k8s_openapi::api::core::v1::Pod;
    use kube::{api::ListParams, Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client.clone(), WORKER_NAMESPACE);

    // Get initial pod count matching deployment selector
    let selector = "app=face-recognition-worker,status=ready";
    let list_params = ListParams::default().labels(selector);

    // Wait for all 3 pods to be ready (max 60 seconds)
    let mut initial_count = 0;
    for attempt in 1..=20 {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let current_pods = pods.list(&list_params).await?;
        let count = current_pods.items.len();
        println!("   Attempt {}: {} ready pods", attempt, count);
        if count == 3 {
            initial_count = count;
            println!("✓ All 3 pods ready");
            break;
        }
        if attempt == 20 {
            return Err(
                format!("Timed out waiting for 3 ready pods (only {} ready)", count).into(),
            );
        }
    }

    let initial_pods = pods.list(&list_params).await?;

    // Select first pod and change its status label to "busy"
    let pod_to_detach = &initial_pods.items[0];
    let pod_name = pod_to_detach.metadata.name.as_ref().unwrap();
    println!("\n3. Detaching pod: {}", pod_name);

    // Patch the pod to change status label
    let patch = serde_json::json!({
        "metadata": {
            "labels": {
                "status": "busy"
            }
        }
    });

    use kube::api::{Patch, PatchParams};
    pods.patch(pod_name, &PatchParams::default(), &Patch::Strategic(patch))
        .await?;
    println!("✓ Changed pod label to status=busy");

    // Wait a moment for deployment controller to react
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify the pod no longer matches deployment selector
    let ready_pods_after = pods.list(&list_params).await?;
    let ready_count_after = ready_pods_after.items.len();
    println!("\n4. Verifying pod detachment...");
    println!("   Ready pods after detachment: {}", ready_count_after);

    // The detached pod should not be in the ready list
    assert!(
        !ready_pods_after
            .items
            .iter()
            .any(|p| { p.metadata.name.as_ref() == Some(pod_name) }),
        "Detached pod should not match deployment selector"
    );

    // Wait for deployment to create replacement pod
    println!("\n5. Waiting for replacement pod...");
    for i in 1..=10 {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let current_pods = pods.list(&list_params).await?;
        let current_count = current_pods.items.len();
        println!("   Attempt {}: {} ready pods", i, current_count);

        if current_count == initial_count {
            println!("✓ Deployment created replacement pod");
            println!("✓ Pod detachment verified successfully");

            // Cleanup: delete deployment
            Command::new("kubectl")
                .args([
                    "delete",
                    "deployment",
                    "worker",
                    "-n",
                    WORKER_NAMESPACE,
                    "--context",
                    "kind-face-it",
                ])
                .output()?;

            return Ok(());
        }
    }

    Err("Deployment did not create replacement pod in time".into())
}
