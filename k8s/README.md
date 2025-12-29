# Kubernetes Manifests

This directory contains Kubernetes manifests for deploying face-it to a cluster.

## Files

1. **namespaces.yaml** - Creates two namespaces:
   - `face-it-api`: API server namespace
   - `face-it-workers`: Worker pods namespace (security boundary)

2. **rbac.yaml** - RBAC configuration:
   - `api-server-sa`: ServiceAccount for API server in face-it-api namespace
   - `worker-sa`: ServiceAccount for workers in face-it-workers namespace
   - `pod-manager-role`: Role allowing pod get/list/patch in face-it-workers
   - `api-server-pod-manager-binding`: Cross-namespace binding allowing API server to manage worker pods

3. **embeddings-secret.yaml** - Biometric embeddings:
   - Secret containing test face embeddings (base64 encoded JSON)
   - Mounted read-only at `/etc/embeddings/data.json` in worker pods
   - Contains 512-dimensional ArcFace embeddings for registered users

4. **worker-deployment.yaml** - Worker pod deployment:
   - 3 replicas (adjust based on load)
   - Uses `face-it-worker:latest` image (build with `./build-worker.sh`)
   - Labels: `app=face-recognition-worker`, `status=ready` (used by pod pool)
   - Mounts embeddings secret
   - Readiness/liveness probes on `/ready` and `/health`
   - Resource limits: 512Mi RAM, 1 CPU

5. **api-server-deployment.yaml** - API server deployment:
   - 2 replicas (horizontal scaling)
   - Uses `face-it-api-server:latest` image
   - Service with LoadBalancer type (change to NodePort/ClusterIP if needed)
   - Communicates with workers via Kubernetes API

## Deployment Order

Apply manifests in this order:

```bash
# 1. Namespaces first
kubectl apply -f k8s/namespaces.yaml

# 2. RBAC (requires namespaces)
kubectl apply -f k8s/rbac.yaml

# 3. Embeddings secret (required by workers)
kubectl apply -f k8s/embeddings-secret.yaml

# 4. Workers (must be ready before API server)
kubectl apply -f k8s/worker-deployment.yaml

# 5. API server (last)
kubectl apply -f k8s/api-server-deployment.yaml
```

## Verification

```bash
# Check all pods are running
kubectl get pods -n face-it-api
kubectl get pods -n face-it-workers

# Check API server logs
kubectl logs -n face-it-api -l app=api-server

# Check worker logs
kubectl logs -n face-it-workers -l app=face-recognition-worker

# Test authentication
kubectl port-forward -n face-it-api svc/api-server 8080:80
curl -X POST http://localhost:8080/api/authenticate \
  -H "Content-Type: application/json" \
  -d '{"image": "'$(base64 < test-data/user1.png)'", "user_id": "user1"}'
```

## Security Model

- **Namespace isolation**: API server in `face-it-api` cannot access the embeddings Secret in `face-it-workers`
- **RBAC**: API server can only get/list/patch pods, not read secrets
- **ServiceAccounts**: Separate service accounts for API server and workers
- **Read-only mount**: Embeddings mounted read-only in worker pods

## Production Notes

For production deployment:

1. **Change image pull policy**: Remove `imagePullPolicy: Never` and push images to a registry
2. **Service type**: Change LoadBalancer to ClusterIP and use Ingress
3. **TLS**: Add Ingress with TLS termination
4. **Secrets management**: Use external secrets manager (e.g., AWS Secrets Manager, HashiCorp Vault)
5. **Resource limits**: Adjust based on actual load and profiling
6. **Replicas**: Scale workers based on authentication load (4-5 req/sec per worker)
7. **Monitoring**: Add Prometheus metrics and alerting
8. **PodDisruptionBudget**: Ensure availability during updates
9. **NetworkPolicy**: Restrict network access between namespaces
10. **Image scanning**: Scan Docker images for vulnerabilities

## Local Development with kind

For local testing with kind cluster:

```bash
# Build images
./build-worker.sh
docker build -t face-it-api-server:latest -f api-server/Dockerfile .

# Load into kind cluster
kind load docker-image face-it-worker:latest --name kind-face-it
kind load docker-image face-it-api-server:latest --name kind-face-it

# Deploy
kubectl apply -f k8s/

# Port forward for testing
kubectl port-forward -n face-it-api svc/api-server 8080:80
```
