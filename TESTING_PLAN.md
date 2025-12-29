# face-it Testing Strategy

## Overview

Comprehensive three-tier testing strategy with 70 tests providing full coverage from unit to end-to-end authentication.

**Test Status**: ✅ **All Passing**
- Unit tests: 64/64 ✅
- Integration tests: 5/5 ✅
- E2E tests: 1/1 ✅
- Total: 70/70 ✅

## Testing Philosophy

### Test Pyramid

```
           E2E (1)
      ┌──────────────┐
      │ Full Auth    │  Real face recognition
      │ ArcFace ONNX │  Complete system
      │ Kind cluster │  Slow but comprehensive
      └──────────────┘

      Integration (5)
    ┌──────────────────┐
    │ Pod Pool + RBAC  │  Real Kubernetes
    │ Optimistic Lock  │  Kind cluster
    │ Cross-namespace  │  Medium speed
    └──────────────────┘

         Unit (64)
  ┌────────────────────────┐
  │ Business Logic         │  Trait mocks
  │ Mock Kubernetes        │  No infrastructure
  │ Fast feedback          │  Sub-second
  └────────────────────────┘
```

### Key Principles

1. **Fast feedback loop**: Unit tests run in <1s without infrastructure
2. **Trait-based DI**: Mock external dependencies (Kubernetes, HTTP)
3. **Idempotent integration**: Kind cluster created once, reused
4. **Real E2E validation**: Production-grade face recognition
5. **Isolation**: Each test cleans up after itself

## Test Levels

### 1. Unit Tests (64 tests)

**Purpose**: Validate business logic without infrastructure

**Coverage**:
- API server: 25 tests
- Worker: 32 tests
- Common: 7 tests

**Approach**:
- Trait-based mocks (`PodOperations`, `HttpClient`)
- In-memory test data
- No network, no Kubernetes, no ONNX model

**Key Unit Tests**:

#### API Server (`api-server/src/`)

**Pod Pool Management** (`kube/pod_manager.rs`):
```rust
#[cfg(test)]
mod tests {
    // Test optimistic locking with resource versions
    #[tokio::test]
    async fn test_detach_pod_success() { /* ... */ }

    #[tokio::test]
    async fn test_detach_pod_conflict_retry() { /* ... */ }

    #[tokio::test]
    async fn test_return_pod_to_pool() { /* ... */ }

    #[tokio::test]
    async fn test_no_available_pods() { /* ... */ }
}
```

**Authentication Handler** (`handlers/authenticate.rs`):
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_authenticate_success() { /* ... */ }

    #[tokio::test]
    async fn test_authenticate_no_pods_available() { /* ... */ }
}
```

**Mock Kubernetes Client** (`kube/mock.rs`):
```rust
pub struct MockPodOperations {
    pods: Arc<Mutex<Vec<Pod>>>,
    patch_should_fail: Arc<Mutex<bool>>,
}

impl PodOperations for MockPodOperations {
    async fn list_pods(&self, selector: &str) -> Result<Vec<Pod>> {
        // In-memory mock
    }

    async fn patch_pod_labels(&self, name: &str, labels: HashMap<String, String>) -> Result<Pod> {
        // Simulate conflicts for testing
    }
}
```

#### Worker (`worker/src/`)

**Face Model** (`face/model.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_face_model_load_placeholder_mode() { /* ... */ }

    #[test]
    fn test_generate_placeholder_embedding() { /* ... */ }

    #[test]
    fn test_preprocess_image() {
        // Validates NHWC format and ArcFace normalization
    }

    #[test]
    fn test_different_images_different_embeddings() { /* ... */ }
}
```

**Face Matcher** (`face/matcher.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_cosine_similarity() { /* ... */ }

    #[test]
    fn test_find_match_success() { /* ... */ }

    #[test]
    fn test_find_match_below_threshold() { /* ... */ }

    #[test]
    fn test_normalized_embedding_magnitude() { /* ... */ }
}
```

**Embeddings Database** (`embeddings/database.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_load_embeddings_success() { /* ... */ }

    #[test]
    fn test_find_user_by_id() { /* ... */ }

    #[test]
    fn test_empty_database() { /* ... */ }
}
```

**Running Unit Tests**:
```bash
# All unit tests (fast, ~1-2 seconds)
cargo test --lib --bins

# Specific crate
cargo test -p api-server --lib
cargo test -p worker --lib
cargo test -p common --lib

# Watch mode for TDD
cargo watch -x "test --lib --bins"
```

### 2. Integration Tests (5 tests)

**Purpose**: Validate Kubernetes interactions with real cluster

**Infrastructure**: Kind cluster (`kind-face-it`)

**Coverage**:
- Pod operations (create, list, patch, delete)
- RBAC (cross-namespace permissions)
- Optimistic locking (resource version conflicts)
- Pod pool detachment and return
- Concurrent access scenarios

**KindCluster Fixture** (`test-utils/src/lib.rs`):
```rust
pub struct KindCluster {
    cluster_name: String,
}

impl KindCluster {
    /// Setup kind cluster (idempotent - reuses existing cluster)
    pub async fn setup() -> Result<Self> {
        // 1. Check if cluster exists
        // 2. Create if missing (kind create cluster)
        // 3. Wait for cluster ready
        // 4. Apply RBAC configuration
        // 5. Create namespaces
    }

    /// Clean up namespaces between tests (keep cluster)
    pub async fn cleanup_namespaces(&self) -> Result<()> {
        // Delete and recreate namespaces for clean state
    }
}
```

**Key Integration Tests** (`api-server/tests/integration_pod_pool.rs`):

```rust
#[tokio::test]
#[ignore] // Requires kind cluster
async fn test_cluster_exists() {
    let cluster = KindCluster::setup().await.unwrap();
    // Validates cluster is running and accessible
}

#[tokio::test]
#[ignore]
async fn test_create_and_list_pods() {
    let cluster = KindCluster::setup().await.unwrap();
    // Create pod in face-it-workers namespace
    // List pods via Kubernetes API
    // Verify pod exists
}

#[tokio::test]
#[ignore]
async fn test_pod_label_patching() {
    let cluster = KindCluster::setup().await.unwrap();
    // Create pod with status=idle
    // Patch to status=busy
    // Verify label changed
    // Patch back to status=idle
}

#[tokio::test]
#[ignore]
async fn test_optimistic_locking_conflict() {
    let cluster = KindCluster::setup().await.unwrap();
    // Create pod
    // Save resource version
    // Modify pod (invalidates resource version)
    // Attempt patch with old resource version
    // Verify 409 Conflict error
}

#[tokio::test]
#[ignore]
async fn test_concurrent_pod_detachment() {
    let cluster = KindCluster::setup().await.unwrap();
    // Create 3 pods with status=idle
    // Apply RBAC (api-server-sa in face-it-api namespace)
    // Detach pod using cross-namespace permissions
    // Verify RBAC allows operation
}
```

**Running Integration Tests**:
```bash
# All integration tests (creates/reuses kind cluster)
cargo test -p api-server --test integration_pod_pool -- --ignored --test-threads=1

# First run: ~2-3 minutes (creates cluster)
# Subsequent runs: ~30 seconds (reuses cluster)

# Clean slate (delete cluster first)
kind delete cluster --name kind-face-it
cargo test -p api-server --test integration_pod_pool -- --ignored --test-threads=1

# Specific test
cargo test -p api-server --test integration_pod_pool test_pod_label_patching -- --ignored
```

**Why --test-threads=1?**
- Integration tests share the kind cluster
- Parallel execution can cause namespace conflicts
- Sequential execution ensures clean state

### 3. End-to-End Tests (1 test)

**Purpose**: Validate complete authentication flow with real face recognition

**Infrastructure**:
- Kind cluster with worker pods
- ArcFace ONNX model (130MB)
- Real face embeddings

**Coverage**:
- Full authentication request/response
- ArcFace inference (512-dim embeddings)
- Cosine similarity matching
- High confidence for matching faces (99.99987%)
- Correct rejection of different faces

**E2E Test** (`api-server/tests/integration_e2e.rs`):

```rust
#[tokio::test]
#[ignore] // Requires kind cluster + worker image
async fn test_e2e_face_authentication() {
    // 1. Setup kind cluster
    let cluster = KindCluster::setup().await.unwrap();

    // 2. Create embeddings Secret from test-data/embeddings.json
    create_embeddings_secret(&cluster).await.unwrap();

    // 3. Deploy worker pod with ArcFace model
    deploy_worker_pod(&cluster).await.unwrap();

    // 4. Wait for worker ready (embeddings loaded)
    wait_for_worker_ready(&cluster).await.unwrap();

    // 5. Deploy API server
    deploy_api_server(&cluster).await.unwrap();

    // 6. Test: Authenticate with user1.png (should match user1)
    let user1_image = include_bytes!("../../../test-data/user1.png");
    let response = authenticate_request(&cluster, user1_image).await.unwrap();

    assert!(response.matched);
    assert_eq!(response.user_id, Some("user1".to_string()));
    assert!(response.confidence > 0.99); // 99.99987% actual

    // 7. Test: Authenticate with different.png (should NOT match)
    let different_image = include_bytes!("../../../test-data/different.png");
    let response = authenticate_request(&cluster, different_image).await.unwrap();

    assert!(!response.matched);
    assert_eq!(response.user_id, None);
}
```

**E2E Test Results**:
```
✅ test_e2e_face_authentication (103.63s)
  - user1.png → matched=true, user_id="user1", confidence=0.9999987
  - different.png → matched=false, user_id=None
```

**Running E2E Tests**:
```bash
# Build worker image first (includes ArcFace model)
./build-worker.sh

# Run E2E test
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1

# Expected output:
# - Cluster setup: ~10s
# - Image load: ~5s
# - Pod startup: ~30s (model loading)
# - Test execution: ~60s (authentication)
# - Total: ~105s
```

## Test Data

### Test Images (`test-data/`)

**Synthetic faces** generated with structural differences:
- `user1.png`: Oval face, eyes 35px apart, nose 35×12px
- `user2.png`: Round face, eyes 40px apart, nose 28×15px
- `user3.png`: Long face, eyes 32px apart, nose 42×10px
- `user1_similar.png`: Similar to user1 (expression change)
- `different.png`: Wide face, eyes 55px apart (extreme), nose 20×20px

**Design principle**: Structural differences only (no color/background tricks)

### Embeddings (`test-data/embeddings.json`)

**Real 512-dimensional embeddings** generated by ArcFace ONNX model:
```json
{
  "embeddings": [
    {
      "user_id": "user1",
      "name": "User One",
      "embedding": [0.123, -0.456, ..., 0.789]
    }
  ]
}
```

**Generation**:
```bash
cd test-data
python generate_real_embeddings.py > embeddings.json
```

## CI/CD Integration

### CI Pipeline (GitHub Actions / GitLab CI)

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      # Fast unit tests (no infrastructure)
      - name: Run unit tests
        run: cargo test --lib --bins
        # ~1-2 seconds

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      # Install kind
      - name: Install kind
        run: |
          curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-linux-amd64
          chmod +x ./kind
          sudo mv ./kind /usr/local/bin/kind

      # Integration tests (creates kind cluster)
      - name: Run integration tests
        run: cargo test -p api-server --test integration_pod_pool -- --ignored --test-threads=1
        # ~2-3 minutes first run

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      # Install kind + kubectl
      - name: Setup Kubernetes
        uses: helm/kind-action@v1.8.0

      # Build worker image with ArcFace model
      - name: Build worker image
        run: ./build-worker.sh
        # Downloads model if missing (~130MB)

      # E2E authentication test
      - name: Run E2E tests
        run: cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
        # ~2-3 minutes
```

### Local Development Workflow

```bash
# 1. TDD with unit tests (fast feedback)
cargo watch -x "test --lib --bins"

# 2. Validate integration before commit
cargo test -p api-server --test integration_pod_pool -- --ignored --test-threads=1

# 3. Full validation before PR
./build-worker.sh
cargo test --all -- --ignored --test-threads=1
```

## Test Maintenance

### Updating Test Data

```bash
cd test-data

# 1. Modify face configurations in generate_synthetic_faces.py
# 2. Generate new images
python generate_synthetic_faces.py

# 3. Generate new embeddings with ArcFace
python generate_real_embeddings.py > embeddings.json

# 4. Rebuild worker image
cd ..
./build-worker.sh

# 5. Run E2E test to verify
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
```

### Adding New Tests

**Unit Test Example**:
```rust
// worker/src/face/model.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // Arrange
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();

        // Act
        let result = model.some_new_method();

        // Assert
        assert_eq!(result, expected);
    }
}
```

**Integration Test Example**:
```rust
// api-server/tests/integration_new_feature.rs
use test_utils::KindCluster;

#[tokio::test]
#[ignore]
async fn test_new_kubernetes_feature() {
    let cluster = KindCluster::setup().await.unwrap();

    // Test new feature with real Kubernetes

    cluster.cleanup_namespaces().await.unwrap();
}
```

## Troubleshooting

### Unit Tests Failing

**Symptom**: Compilation errors or test failures
```bash
# Check for compilation issues
cargo check --all-targets

# Run specific failing test with output
cargo test --lib test_name -- --nocapture
```

### Integration Tests Failing

**Symptom**: "kind cluster not found" or namespace errors
```bash
# Delete and recreate cluster
kind delete cluster --name kind-face-it
cargo test -p api-server --test integration_pod_pool -- --ignored --test-threads=1

# Check cluster status
kind get clusters
kubectl cluster-info --context kind-kind-face-it

# Check RBAC
kubectl get sa -n face-it-api
kubectl get role -n face-it-workers
kubectl get rolebinding -n face-it-workers
```

### E2E Tests Failing

**Symptom**: Authentication returns low confidence or incorrect results

```bash
# 1. Verify worker image has model
docker run --rm face-it-worker:latest ls -lh /models/face_recognition.onnx
# Should show ~130MB file

# 2. Check worker logs
kubectl logs -n face-it-workers <pod-name>
# Should NOT see "using placeholder mode" warning

# 3. Verify embeddings Secret exists
kubectl get secret -n face-it-workers embeddings -o yaml

# 4. Rebuild everything
./build-worker.sh
kind load docker-image face-it-worker:latest --name kind-face-it
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
```

## Performance Benchmarks

### Test Execution Times

```
Unit Tests (64):        ~1-2 seconds
Integration Tests (5):  ~30-120 seconds
  - First run: ~120s (cluster creation)
  - Subsequent: ~30s (reuses cluster)
E2E Tests (1):         ~105 seconds
  - Cluster setup: ~10s
  - Image load: ~5s
  - Pod startup: ~30s
  - Authentication: ~60s

Total (all tests):     ~135 seconds
```

### Optimization Tips

1. **Reuse kind cluster**: Don't delete between test runs
2. **Parallel unit tests**: Default cargo test behavior
3. **Sequential integration**: Required for shared cluster
4. **Cache Docker images**: kind load once, reuse
5. **Pre-download model**: Run `./build-worker.sh` once

## Summary

**Testing Coverage**:
- ✅ 70/70 tests passing
- ✅ Unit tests for all business logic
- ✅ Integration tests for Kubernetes operations
- ✅ E2E test for complete authentication flow
- ✅ Real face recognition validation (99.99987% confidence)

**Key Achievements**:
- Fast feedback with unit tests (<2s)
- Idempotent integration test infrastructure
- Production-grade E2E validation
- Trait-based DI for testability
- Automated build and test workflow

**Testing Philosophy**:
- Test at appropriate level (unit > integration > E2E)
- Mock external dependencies for unit tests
- Use real infrastructure for integration tests
- Validate complete system with E2E tests
- Maintain fast feedback loop for development
