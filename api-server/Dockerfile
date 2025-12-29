# Build stage
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY api-server ./api-server
COPY worker ./worker
COPY test-utils ./test-utils

# Build the api-server binary in release mode
RUN cargo build --release -p api-server

# Runtime stage - use latest Debian to match rust:slim
FROM debian:latest

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 libgcc-s1 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /build/target/release/api-server /usr/local/bin/api-server

# Set environment variables with defaults
ENV PORT=8080 \
    WORKER_NAMESPACE=face-it-workers \
    WORKER_SELECTOR=app=face-worker,status=ready \
    RUST_LOG=info

# Expose the API server port
EXPOSE 8080

# Run the API server
ENTRYPOINT ["/usr/local/bin/api-server"]
