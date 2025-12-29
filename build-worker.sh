#!/bin/bash
set -euo pipefail

# Build script for face-it worker Docker image
# - Downloads ArcFace model if missing
# - Builds Docker image
# - Optionally loads into kind cluster

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODEL_PATH="$SCRIPT_DIR/worker/models/arcface.onnx"
MODEL_URL="https://huggingface.co/garavv/arcface-onnx/resolve/main/arcface.onnx"
IMAGE_NAME="face-it-worker:latest"
KIND_CLUSTER="kind-face-it"

echo "==> Building face-it worker image"
echo

# Check if model exists
if [ ! -f "$MODEL_PATH" ]; then
    echo "⚠️  ArcFace model not found at: $MODEL_PATH"
    echo "📥 Downloading model (130MB)..."
    echo "   Source: $MODEL_URL"
    echo

    mkdir -p "$(dirname "$MODEL_PATH")"

    if command -v curl >/dev/null 2>&1; then
        curl -L --progress-bar -o "$MODEL_PATH" "$MODEL_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget --show-progress -O "$MODEL_PATH" "$MODEL_URL"
    else
        echo "❌ Error: curl or wget required to download model"
        exit 1
    fi

    echo "✅ Model downloaded successfully"
    echo
else
    echo "✅ ArcFace model found at: $MODEL_PATH"
    MODEL_SIZE=$(du -h "$MODEL_PATH" | cut -f1)
    echo "   Size: $MODEL_SIZE"
    echo
fi

# Build Docker image
echo "🐳 Building Docker image: $IMAGE_NAME"
echo
docker build -t "$IMAGE_NAME" -f worker/Dockerfile .

echo
echo "✅ Docker image built successfully: $IMAGE_NAME"

# Check if kind cluster exists and offer to load
if command -v kind >/dev/null 2>&1; then
    if kind get clusters 2>/dev/null | grep -q "^${KIND_CLUSTER}$"; then
        echo
        echo "📦 Loading image into kind cluster: $KIND_CLUSTER"
        kind load docker-image "$IMAGE_NAME" --name "$KIND_CLUSTER"
        echo "✅ Image loaded into kind cluster"
    else
        echo
        echo "ℹ️  Kind cluster '$KIND_CLUSTER' not found (skipping load)"
        echo "   To load manually: kind load docker-image $IMAGE_NAME --name $KIND_CLUSTER"
    fi
fi

echo
echo "🎉 Build complete!"
