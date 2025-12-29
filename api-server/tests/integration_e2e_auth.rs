use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;
/// End-to-end integration test for face authentication
/// Tests the complete flow: API server -> worker -> face recognition -> response
///
/// Run with: cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
use test_utils::{KindCluster, WORKER_NAMESPACE};

const TEST_DATA_DIR: &str = "../test-data";

/// Setup function that runs before the test
fn setup() -> KindCluster {
    KindCluster::setup().expect("Failed to setup kind cluster")
}

/// Read and base64-encode an image file
fn read_image_as_base64(path: &str) -> Result<String> {
    let image_bytes = fs::read(path).context(format!("Failed to read image: {}", path))?;
    Ok(general_purpose::STANDARD.encode(&image_bytes))
}

/// Create Kubernetes Secret with embeddings data
async fn create_embeddings_secret() -> Result<()> {
    use k8s_openapi::api::core::v1::Secret;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let secrets: Api<Secret> = Api::namespaced(client, WORKER_NAMESPACE);

    // Read embeddings JSON
    let embeddings_path = format!("{}/embeddings.json", TEST_DATA_DIR);
    let embeddings_data = fs::read(&embeddings_path).context(format!(
        "Failed to read embeddings from {}",
        embeddings_path
    ))?;

    // Create secret
    let secret = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": "face-embeddings",
            "namespace": WORKER_NAMESPACE,
        },
        "data": {
            "data.json": general_purpose::STANDARD.encode(&embeddings_data),
        },
    });

    // Delete existing secret if it exists
    let _ = secrets.delete("face-embeddings", &Default::default()).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    secrets
        .create(&Default::default(), &serde_json::from_value(secret)?)
        .await
        .context("Failed to create embeddings secret")?;

    println!("✓ Created embeddings secret");
    Ok(())
}

/// Deploy worker pods with the face recognition service
async fn deploy_worker_pods(count: usize) -> Result<Vec<String>> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, WORKER_NAMESPACE);

    let mut pod_names = Vec::new();

    for i in 0..count {
        let pod_name = format!("test-worker-{}", i);
        pod_names.push(pod_name.clone());

        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "face-worker".to_string());
        labels.insert("status".to_string(), "ready".to_string());

        // Deploy actual worker with face recognition
        let pod = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": pod_name,
                "labels": labels,
            },
            "spec": {
                "containers": [{
                    "name": "worker",
                    "image": "face-it-worker:latest",
                    "imagePullPolicy": "Never", // Image loaded into kind
                    "ports": [{"containerPort": 8080}],
                    "env": [
                        {"name": "PORT", "value": "8080"},
                        {"name": "EMBEDDINGS_PATH", "value": "/etc/embeddings/data.json"},
                        {"name": "RUST_LOG", "value": "info"},
                    ],
                    "volumeMounts": [{
                        "name": "embeddings",
                        "mountPath": "/etc/embeddings",
                        "readOnly": true,
                    }],
                    "readinessProbe": {
                        "httpGet": {
                            "path": "/health",
                            "port": 8080,
                        },
                        "initialDelaySeconds": 2,
                        "periodSeconds": 1,
                    },
                }],
                "volumes": [{
                    "name": "embeddings",
                    "secret": {
                        "secretName": "face-embeddings"
                    }
                }]
            },
        });

        pods.create(&Default::default(), &serde_json::from_value(pod)?)
            .await
            .context(format!("Failed to create pod {}", pod_name))?;

        println!("✓ Created worker pod: {}", pod_name);
    }

    // Wait for pods to be ready
    for pod_name in &pod_names {
        wait_for_pod_ready(WORKER_NAMESPACE, pod_name).await?;
        println!("✓ Worker pod ready: {}", pod_name);
    }

    Ok(pod_names)
}

/// Wait for pod to be ready
async fn wait_for_pod_ready(namespace: &str, name: &str) -> Result<()> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};
    use std::time::Duration;
    use tokio::time::sleep;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    for _ in 0..60 {
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

/// Deploy API server pod
async fn deploy_api_server() -> Result<String> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    const API_NAMESPACE: &str = "face-it-api";

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, API_NAMESPACE);

    let pod_name = "test-api-server".to_string();

    let pod = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": &pod_name,
            "labels": {
                "app": "face-api-server"
            },
        },
        "spec": {
            "serviceAccountName": "api-server-sa",
            "containers": [{
                "name": "api-server",
                "image": "face-it-api-server:latest",
                "imagePullPolicy": "Never", // Image loaded into kind
                "ports": [{"containerPort": 8080}],
                "env": [
                    {"name": "PORT", "value": "8080"},
                    {"name": "WORKER_NAMESPACE", "value": WORKER_NAMESPACE},
                    {"name": "WORKER_SELECTOR", "value": "app=face-worker,status=ready"},
                    {"name": "RUST_LOG", "value": "info"},
                ],
                "readinessProbe": {
                    "httpGet": {
                        "path": "/health",
                        "port": 8080,
                    },
                    "initialDelaySeconds": 2,
                    "periodSeconds": 1,
                },
            }],
        },
    });

    pods.create(&Default::default(), &serde_json::from_value(pod)?)
        .await
        .context("Failed to create API server pod")?;

    println!("✓ Created API server pod: {}", pod_name);

    // Wait for API server to be ready
    wait_for_pod_ready(API_NAMESPACE, &pod_name).await?;
    println!("✓ API server pod ready: {}", pod_name);

    Ok(pod_name)
}

/// Send authentication request to API server using kubectl exec
async fn send_auth_request(api_pod_name: &str, image_base64: String) -> Result<serde_json::Value> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};
    use std::io::Write;
    use std::process::{Command, Stdio};

    const API_NAMESPACE: &str = "face-it-api";

    // Get API server pod IP
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, API_NAMESPACE);
    let api_pod = pods.get(api_pod_name).await?;
    let api_pod_ip = api_pod
        .status
        .and_then(|s| s.pod_ip)
        .context("API server pod has no IP")?;

    println!("  API server IP: {}", api_pod_ip);

    // Create request JSON
    let request_body = serde_json::json!({
        "image_base64": image_base64,
    });
    let request_json = serde_json::to_string(&request_body)?;

    // Send request from within a worker pod using kubectl exec + curl
    // Use stdin to pass the large request body
    let mut child = Command::new("kubectl")
        .args([
            "--context",
            "kind-face-it",
            "-n",
            WORKER_NAMESPACE,
            "exec",
            "-i",
            "test-worker-0",
            "--",
            "curl",
            "-s",
            "-X",
            "POST",
            &format!("http://{}:8080/authenticate", api_pod_ip),
            "-H",
            "Content-Type: application/json",
            "-d",
            "@-", // Read from stdin
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn kubectl exec curl")?;

    // Write request to stdin
    {
        let stdin = child.stdin.as_mut().context("Failed to open stdin")?;
        stdin
            .write_all(request_json.as_bytes())
            .context("Failed to write to stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for kubectl exec")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "kubectl exec curl failed\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        );
    }

    let body = String::from_utf8(output.stdout).context("Failed to parse response as UTF-8")?;

    println!("  Response body: {}", body);

    serde_json::from_str(&body).context("Failed to parse response JSON")
}

/// Cleanup test resources
async fn cleanup() -> Result<()> {
    use k8s_openapi::api::core::v1::{Pod, Secret};
    use kube::{Api, Client};

    const API_NAMESPACE: &str = "face-it-api";

    let client = Client::try_default().await?;
    let api_pods: Api<Pod> = Api::namespaced(client.clone(), API_NAMESPACE);
    let worker_pods: Api<Pod> = Api::namespaced(client.clone(), WORKER_NAMESPACE);
    let secrets: Api<Secret> = Api::namespaced(client, WORKER_NAMESPACE);

    // Delete API server pod
    let _ = api_pods
        .delete("test-api-server", &Default::default())
        .await;

    // Delete worker pods
    for i in 0..3 {
        let pod_name = format!("test-worker-{}", i);
        let _ = worker_pods.delete(&pod_name, &Default::default()).await;
    }

    // Delete secret
    let _ = secrets.delete("face-embeddings", &Default::default()).await;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_e2e_face_authentication() -> Result<()> {
    println!("\n=== End-to-End Face Authentication Test ===\n");

    let _cluster = setup();

    // Cleanup any previous test resources
    let _ = cleanup().await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Step 1: Create embeddings secret
    println!("\n1. Creating embeddings secret...");
    create_embeddings_secret().await?;

    // Step 2: Apply RBAC configuration
    println!("\n2. Applying RBAC configuration...");
    _cluster.apply_rbac()?;

    // Step 3: Deploy worker pods
    println!("\n3. Deploying worker pods...");
    let worker_pods = deploy_worker_pods(3).await?;
    assert_eq!(worker_pods.len(), 3, "Should deploy 3 worker pods");

    // Step 4: Deploy API server
    println!("\n4. Deploying API server...");
    let api_pod_name = deploy_api_server().await?;

    // Step 5: Test authentication with exact image (should succeed)
    println!("\n5. Testing authentication with exact image (should match user1)...");
    let similar_image = read_image_as_base64(&format!("{}/user1.png", TEST_DATA_DIR))?;

    let response = send_auth_request(&api_pod_name, similar_image).await?;

    // Verify response structure
    assert!(
        response["matched"].as_bool().unwrap(),
        "Similar image should match"
    );
    assert_eq!(
        response["user_id"].as_str().unwrap(),
        "user1",
        "Should match user1"
    );
    assert!(
        response["confidence"].as_f64().unwrap() >= 0.7,
        "Confidence should be >= 0.7"
    );

    println!("✓ Similar image authenticated successfully:");
    println!("  - User: {}", response["user_name"].as_str().unwrap());
    println!(
        "  - Confidence: {:.2}",
        response["confidence"].as_f64().unwrap()
    );

    // Step 6: Test authentication with different image (should fail)
    println!("\n6. Testing authentication with different image (should NOT match)...");
    let different_image = read_image_as_base64(&format!("{}/different.png", TEST_DATA_DIR))?;

    let response = send_auth_request(&api_pod_name, different_image).await?;

    // Verify response indicates no match
    assert!(
        !response["matched"].as_bool().unwrap(),
        "Different image should NOT match"
    );

    println!("✓ Different image correctly rejected (no match)");

    println!("\n✓ End-to-end authentication test passed!");

    // Cleanup
    println!("\n7. Cleaning up test resources...");
    cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_embeddings_generation() -> Result<()> {
    println!("\n=== Test Embeddings File ===\n");

    let embeddings_path = format!("{}/embeddings.json", TEST_DATA_DIR);
    let embeddings_content =
        fs::read_to_string(&embeddings_path).context("Failed to read embeddings.json")?;

    let embeddings: serde_json::Value =
        serde_json::from_str(&embeddings_content).context("Failed to parse embeddings.json")?;

    // Verify structure
    assert!(
        embeddings["embeddings"].is_array(),
        "Should have embeddings array"
    );

    let emb_array = embeddings["embeddings"].as_array().unwrap();
    assert_eq!(emb_array.len(), 3, "Should have 3 user embeddings");

    // Verify each embedding
    for (i, user_emb) in emb_array.iter().enumerate() {
        assert!(
            user_emb["user_id"].is_string(),
            "User {} should have user_id",
            i
        );
        assert!(user_emb["name"].is_string(), "User {} should have name", i);
        assert!(
            user_emb["embedding"].is_array(),
            "User {} should have embedding array",
            i
        );

        let embedding = user_emb["embedding"].as_array().unwrap();
        assert_eq!(
            embedding.len(),
            512,
            "User {} should have 512-dim embedding",
            i
        );

        println!(
            "✓ User {}: {} ({})",
            i + 1,
            user_emb["user_id"].as_str().unwrap(),
            user_emb["name"].as_str().unwrap()
        );
    }

    println!("\n✓ Embeddings file structure verified!");

    Ok(())
}
