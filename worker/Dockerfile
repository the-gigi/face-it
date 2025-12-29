# Build stage
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev g++ && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY worker ./worker
COPY api-server ./api-server
COPY test-utils ./test-utils

# Build the worker binary in release mode
RUN cargo build --release -p worker

# Runtime stage - use Debian slim for compatibility
FROM debian:12-slim

# Install runtime dependencies including curl for testing
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /build/target/release/worker /usr/local/bin/worker

# Copy ArcFace model
COPY worker/models/arcface.onnx /models/face_recognition.onnx

# Set environment variables with defaults
ENV PORT=8080 \
    EMBEDDINGS_PATH=/etc/embeddings/data.json \
    MODEL_PATH=/models/face_recognition.onnx \
    MATCH_THRESHOLD=0.7 \
    RUST_LOG=info

# Expose the worker port
EXPOSE 8080

# Run the worker
ENTRYPOINT ["/usr/local/bin/worker"]
