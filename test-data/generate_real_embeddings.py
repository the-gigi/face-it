#!/usr/bin/env python3
"""Generate real face embeddings using ArcFace ONNX model.

This script loads test face images and generates 512-dimensional embeddings
using the ArcFace face recognition model.
"""

import json
import numpy as np
import onnxruntime as ort
from PIL import Image

# Model path
MODEL_PATH = "../worker/models/arcface.onnx"

# Test images
IMAGES = [
    ("user1.png", "user1", "User One"),
    ("user2.png", "user2", "User Two"),
    ("user3.png", "user3", "User Three"),
]


def preprocess_image(image_path: str) -> np.ndarray:
    """Preprocess image for ArcFace model.

    ArcFace expects:
    - Input shape: (1, 112, 112, 3)
    - Pixel range: [-1, 1] with normalization (pixel - 127.5) / 128.0
    - RGB format
    """
    # Load and resize image
    img = Image.open(image_path).convert('RGB')
    img = img.resize((112, 112), Image.Resampling.BILINEAR)

    # Convert to numpy array
    img_array = np.array(img, dtype=np.float32)

    # Normalize: (pixel - 127.5) / 128.0
    img_array = (img_array - 127.5) / 128.0

    # Add batch dimension: (112, 112, 3) -> (1, 112, 112, 3)
    img_array = np.expand_dims(img_array, axis=0)

    return img_array


def generate_embedding(sess: ort.InferenceSession, image_path: str) -> list:
    """Generate 512-dimensional embedding from image."""
    # Preprocess image
    input_data = preprocess_image(image_path)

    # Get input/output names
    input_name = sess.get_inputs()[0].name
    output_name = sess.get_outputs()[0].name

    # Run inference
    embedding = sess.run([output_name], {input_name: input_data})[0][0]

    # Convert to list for JSON serialization
    return embedding.tolist()


def main():
    import sys

    print("Loading ArcFace ONNX model...", file=sys.stderr)
    sess = ort.InferenceSession(MODEL_PATH)

    print(f"Model loaded successfully!", file=sys.stderr)
    print(f"Input shape: {sess.get_inputs()[0].shape}", file=sys.stderr)
    print(f"Output shape: {sess.get_outputs()[0].shape}", file=sys.stderr)
    print(file=sys.stderr)

    embeddings_data = {"embeddings": []}

    for filename, user_id, name in IMAGES:
        print(f"Processing {filename}...", file=sys.stderr)
        embedding = generate_embedding(sess, filename)
        print(f"  Generated {len(embedding)}-dimensional embedding", file=sys.stderr)

        embeddings_data["embeddings"].append({
            "user_id": user_id,
            "name": name,
            "embedding": embedding
        })

    # Output JSON to stdout only
    print(json.dumps(embeddings_data, indent=2))

    print(file=sys.stderr)
    print(f"✓ Generated {len(IMAGES)} real face embeddings using ArcFace model", file=sys.stderr)


if __name__ == "__main__":
    main()
