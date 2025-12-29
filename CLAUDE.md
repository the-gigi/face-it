# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Critical Guideline

Whenever finishing a major task run all tests and verify they pass.

## Project Overview

**face-it** is a Rust-based biometric authentication service demonstrating the pod pool pattern with Kubernetes. The system uses face recognition with in-memory embeddings for fast authentication, featuring node-level isolation for sensitive data processing.

Key architectural pattern: API servers manage a pool of pre-warmed worker pods using optimistic locking. Workers load biometric embeddings at startup and perform face matching without exposing raw data.

## Build and Test Commands

```bash
# Build entire workspace
cargo build
cargo build --release

# Build specific crates
cargo build -p api-server
cargo build -p worker

# Run specific binaries
cargo run -p api-server
cargo run -p worker

# Build Docker images
./build-worker.sh  # Downloads ArcFace model (130MB), builds worker image
docker build -t face-it-api-server:latest -f api-server/Dockerfile .

# Load images into kind cluster (REQUIRED for E2E tests)
# Note: kind cluster name is "face-it" (not "kind-face-it")
kind load docker-image face-it-worker:latest --name face-it
kind load docker-image face-it-api-server:latest --name face-it

# Unit tests (fast, no cluster required)
cargo test --lib --bins

# Integration tests (requires kind cluster)
# Note: Integration tests create/reuse a kind cluster automatically
# Cluster name: "face-it", kubectl context: "kind-face-it"
# Use --test-threads=1 to prevent parallel test conflicts
cargo test --test integration_pod_pool -- --ignored --test-threads=1

# E2E authentication tests (requires kind cluster with both images loaded)
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1

# Run specific integration test
cargo test --test integration_pod_pool test_pod_label_patching -- --ignored

# All tests (unit + integration + E2E)
cargo test --all -- --ignored --test-threads=1

# Code formatting
cargo fmt
cargo fmt --check

# Linting
cargo clippy
cargo clippy -- -D warnings

# Delete test cluster (if needed)
kind delete cluster --name kind-face-it
```

## Project Structure

Cargo workspace with three crates:

- **api-server**: HTTP API server managing pod pool using Kubernetes API
  - Pod pool management with optimistic locking to prevent race conditions
  - Cross-namespace RBAC (manages pods in face-it-workers namespace)
  - Never accesses biometric data directly

- **worker**: Face recognition and matching service
  - Loads embeddings from Kubernetes Secret at startup
  - Performs in-memory face matching using cosine similarity
  - Returns only authentication results (never raw embeddings)

- **common**: Shared types and utilities (AuthRequest, AuthResponse, UserEmbedding)

## Module Organization

Uses modern Rust 2018+ pattern (NOT legacy mod.rs):

```
src/
├── main.rs
├── handlers.rs       # Module declaration
└── handlers/         # Submodule implementations
    ├── authenticate.rs
    └── health.rs
```

Key points:
- Module declaration file (e.g., `handlers.rs`) declares submodules
- Submodule directory (e.g., `handlers/`) contains implementations
- Use `pub use` in declaration files for convenient re-exports
- Simple modules without submodules are just `.rs` files (e.g., `config.rs`)

## Testing Architecture

Three-tier testing strategy:

1. **Unit Tests** (`cargo test --lib --bins`):
   - Use trait abstraction for dependency injection
   - Mock implementations for Kubernetes operations
   - No real cluster required
   - Tests in same file as code (`#[cfg(test)] mod tests`)

2. **Integration Tests** (`cargo test --test integration_* -- --ignored`):
   - Real Kubernetes cluster using kind
   - KindCluster fixture manages cluster lifecycle idempotently
   - Cluster reused across test runs for speed
   - Tests in `tests/` directory
   - Requires `--ignored` flag and `--test-threads=1`

3. **Integration Test Cluster**:
   - Managed by `tests/common/mod.rs::KindCluster`
   - Cluster name: `kind-face-it`
   - Creates cluster on first run, reuses on subsequent runs
   - Cleans namespaces between tests for fresh state
   - All cluster management done in Rust (no shell scripts)

## Key Design Patterns

**Optimistic Locking for Pod Pool**:
- Use Kubernetes resource versions for compare-and-swap
- Multiple API server instances can safely compete for pods
- Retry loop handles conflicts when race conditions occur
- No external coordination (Redis, etcd) needed

**Trait-Based Dependency Injection**:
- Abstract external dependencies (Kubernetes, HTTP) behind traits
- Production implementations use real clients
- Test implementations use in-memory mocks
- Enables unit testing without infrastructure

**Cross-Namespace Security**:
- API server in `face-it-api` namespace (no secret access)
- Workers in `face-it-workers` namespace (has secret access)
- API server has RBAC to manage worker pods but cannot read secrets
- Principle of least privilege enforced at namespace level

## Common Development Patterns

**Error Handling**:
- Each crate has `error.rs` with `thiserror`
- Implement `IntoResponse` for Axum error types
- Use `ApiResult<T>` and `WorkerResult<T>` type aliases

**Shared State**:
- Wrap in `Arc<T>` for thread-safe sharing
- Pass via Axum `Extension` layer
- Example: `Arc<PodManager>`, `Arc<EmbeddingsDatabase>`

**Async/Await**:
- Use `tokio` runtime with `#[tokio::main]`
- Async file I/O with `tokio::fs`
- Async HTTP with `reqwest` (api-server) and `axum` (both)

**Configuration**:
- Environment variables via `Config::from_env()`
- Provide sensible defaults
- Use `#[derive(Clone)]` for sharing across threads

## Namespace Configuration

The project uses two Kubernetes namespaces for security isolation:

- `face-it-api`: API server deployment
- `face-it-workers`: Worker pods with access to embeddings Secret

Integration tests use the same namespace names with the `kind-face-it` cluster.

## Storage Strategy

Biometric embeddings are stored in Kubernetes Secret:
- Mounted as read-only volume at `/etc/embeddings/data.json`
- Workers load all embeddings into memory at startup
- Fast in-memory authentication with cosine similarity
- Suitable for ~1000-2000 users (512-dimensional embeddings)
- JSON format: `{"embeddings": [{"user_id": "...", "name": "...", "embedding": [0.1, ...]}]}`

## Running Integration Tests

Integration tests create/manage a kind cluster automatically:

```bash
# First run: creates kind-face-it cluster
cargo test --test integration_pod_pool -- --ignored --test-threads=1

# Subsequent runs: reuses existing cluster (fast)
cargo test --test integration_pod_pool -- --ignored --test-threads=1

# To start fresh, delete cluster first:
kind delete cluster --name kind-face-it
cargo test --test integration_pod_pool -- --ignored --test-threads=1
```

The `KindCluster::setup()` fixture is idempotent - safe to call multiple times.

## Kubernetes RBAC

API server requires cross-namespace permissions:

```yaml
# In face-it-api namespace
ServiceAccount: api-server-sa

# Role in face-it-workers namespace
Role: pod-manager-role
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list", "patch"]

# Cross-namespace binding
RoleBinding: api-server-pod-manager-binding
  subjects: [{kind: ServiceAccount, name: api-server-sa, namespace: face-it-api}]
  roleRef: {kind: Role, name: pod-manager-role}
```

Integration tests apply this RBAC via `cluster.apply_rbac()`.

## Face Recognition Implementation

Real face recognition using ArcFace ONNX model in `worker/src/face/model.rs`:
- **Model**: ArcFace ResNet from [Hugging Face](https://huggingface.co/garavv/arcface-onnx) (130MB)
- **ONNX Runtime**: Via `ort` crate (2.0 API)
- **Input**: (1, 112, 112, 3) NHWC format, normalized to [-1, 1]
- **Output**: 512-dimensional L2-normalized embeddings
- **Matching**: Cosine similarity with configurable threshold
- **Distribution**: Model in `.gitignore`, baked into Docker image via `./build-worker.sh`
- **Fallback**: Placeholder mode if model file not found (development only)

## CI/CD Considerations

- Unit tests run in CI without infrastructure
- Integration tests require Docker + kind installation
- Tests use `#[ignore]` attribute - must explicitly enable with `--ignored`
- Use `--test-threads=1` for integration tests to avoid conflicts
- Verify CI checks pass after making changes
