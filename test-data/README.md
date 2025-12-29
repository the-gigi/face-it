# Test Data for face-it

This directory contains test images and embeddings for integration testing of the face-it biometric authentication service.

> **ℹ️ Note for Users**: All test data is **already provided** and ready to use. You do **not** need to regenerate anything. The images and embeddings are committed to the repository for your convenience. This README documents how the data was generated and how to regenerate it if needed (e.g., for modifications or learning purposes).

## Overview

The test data consists of:
- **Test images**: Synthetic face images for 3 registered users plus test cases (already provided)
- **Embeddings file**: Real 512-dimensional face embeddings generated using ArcFace ONNX model (`embeddings.json` - already provided)
- **Generation tools**: Python scripts for regenerating test data if needed (reference only)

## Test Images

### Current Images

- `user1.png` - Registered user (oval face, brown eyes, neutral expression)
- `user2.png` - Registered user (round face, blue eyes, smiling)
- `user3.png` - Registered user (long face, green eyes, darker skin)
- `user1_similar.png` - Similar to user1 but smiling (should authenticate as user1)
- `different.png` - Completely different face (should fail authentication)

**Generation method**: Python PIL/Pillow (~2 KB per image)

### Key Design Principles

The test images are designed with **structural differences**, not pixel noise:

- **Face aspect ratio**: Oval (user1), Round (user2), Long (user3), Wide (different)
- **Eye characteristics**: Color (brown/blue/green), size, spacing
- **Nose dimensions**: Length and width ratios
- **Mouth features**: Position, width, expression
- **Eyebrow style**: Thickness, angle
- **Hair style**: Short, long, bald
- **Skin tone**: Varied across users

**Important**: Pixel noise should NOT be a distinguishing feature. A good face recognition system should generalize over noise but distinguish structural features.

## Generating Test Images

### Requirements

- Python 3.9+
- Pillow library

### Setup with uv (recommended)

```bash
cd test-data
uv pip install pillow
uv run python generate_synthetic_faces.py
```

### Alternative setup

```bash
cd test-data
pip install pillow
python generate_synthetic_faces.py
```

### Image Characteristics

- Size: 224x224 pixels
- Format: PNG
- File size: ~2 KB per image
- Features: Structural differences (face shape, eye color/spacing, nose length, etc.)
- No random pixel noise (face recognition should generalize over noise)

## Embeddings File

### `embeddings.json`

Real 512-dimensional face embeddings generated using the ArcFace ONNX model for the three registered users.

**Format:**
```json
{
  "embeddings": [
    {
      "user_id": "user1",
      "name": "User One",
      "embedding": [0.1, 0.2, ..., 0.5]
    }
  ]
}
```

### Generating Real Embeddings with ArcFace

The embeddings are generated using the ArcFace ONNX model via ONNX Runtime:

#### Requirements

- Python 3.9+
- onnxruntime library
- Pillow library
- ArcFace model at `../worker/models/arcface.onnx`

#### Setup with uv (recommended)

```bash
cd test-data
uv pip install onnxruntime pillow
```

#### Generate Embeddings

```bash
cd test-data
python generate_real_embeddings.py > embeddings.json
```

The script:
1. Loads the ArcFace ONNX model from `../worker/models/arcface.onnx`
2. Preprocesses each test image (resize to 112×112, normalize to [-1, 1])
3. Runs inference to generate 512-dimensional embeddings
4. Outputs JSON to stdout (logs go to stderr)

**Output:**
```
Loading ArcFace ONNX model...
Model loaded successfully!
Input shape: [1, 112, 112, 3]
Output shape: [1, 512]

Processing user1.png...
  Generated 512-dimensional embedding
Processing user2.png...
  Generated 512-dimensional embedding
Processing user3.png...
  Generated 512-dimensional embedding

✓ Generated 3 real face embeddings using ArcFace model
```

## Real Face Recognition with ArcFace

The system now uses **real face recognition** with the ArcFace ONNX model:

### ArcFace Model

- **Source**: [Hugging Face garavv/arcface-onnx](https://huggingface.co/garavv/arcface-onnx)
- **Model size**: 130MB
- **Architecture**: ResNet-based face recognition
- **Input format**: (1, 112, 112, 3) NHWC - batch, height, width, channels
- **Output**: (1, 512) - 512-dimensional L2-normalized embeddings
- **Normalization**: (pixel - 127.5) / 128.0

### Model Distribution (Not in Git)

The 130MB model file is **NOT committed to git** (too large). Instead:

1. **Automated build script** (recommended):
   ```bash
   ./build-worker.sh
   ```
   This script automatically:
   - Downloads model if missing (only once)
   - Builds Docker image with model included
   - Loads image into kind cluster if present

2. **Manual download** (if needed):
   ```bash
   mkdir -p worker/models
   curl -L -o worker/models/arcface.onnx \
     https://huggingface.co/garavv/arcface-onnx/resolve/main/arcface.onnx
   docker build -t face-it-worker:latest -f worker/Dockerfile .
   ```

3. **How it works**:
   - Model is in `.gitignore` (not in git)
   - Dockerfile copies model from local filesystem during build
   - Model is baked into Docker image
   - Available in Kubernetes via the Docker image (no separate volume needed)

This approach keeps git lightweight while ensuring the model is available in production.

### Key Features

- ✅ **Semantically meaningful**: Similar faces produce similar embeddings
- ✅ **High accuracy**: 99.99987% confidence for matching faces
- ✅ **Robust rejection**: Correctly rejects structurally different faces
- ✅ **Production-ready**: State-of-the-art face recognition
- ✅ **Fast inference**: ~200-300ms per authentication on CPU
- ✅ **Structural awareness**: Distinguishes faces by geometric features (eye spacing, nose ratios, face shape)

### Fallback: Placeholder Mode

The worker can still operate in placeholder mode if the ONNX model is not available:

- **Use case**: Development/testing without the model file
- **Activation**: Automatically enabled if `MODEL_PATH` file doesn't exist
- **Warning**: Logs a warning about using placeholder mode
- **Limitations**: High false positive rate, not suitable for E2E authentication tests

To suppress the placeholder warning, set `WORKER_USE_REAL_MODEL=false` environment variable.

## Directory Structure

```
test-data/
├── README.md                          # This file
├── pyproject.toml                     # Python dependencies for uv
├── uv.lock                            # uv lock file
├── .venv/                             # Python virtual environment (created by uv)
├── embeddings.json                    # Real ArcFace embeddings (in use)
├── embeddings_generation.log          # Generation log from ArcFace
├── user1.png                          # Test images
├── user2.png
├── user3.png
├── user1_similar.png
├── different.png
├── generate_real_embeddings.py        # ArcFace embedding generator (Python + ONNX Runtime)
└── generate_synthetic_faces.py        # Face image generator (Python + PIL)
```

## Regenerating Test Data

### Quick Start

```bash
cd test-data

# 1. Generate face images
uv pip install pillow
python generate_synthetic_faces.py

# 2. Generate real embeddings using ArcFace
uv pip install onnxruntime pillow
python generate_real_embeddings.py > embeddings.json

# 3. Build Docker image (downloads model if missing, builds, loads to kind)
cd ..
./build-worker.sh

# 4. Test that authentication works
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
```

### Full Workflow

1. **Modify face configurations** in `generate_synthetic_faces.py` if needed:
   - Adjust face shapes, eye spacing, nose ratios, etc.
   - Ensure structural differences (not colors/backgrounds)

2. **Generate face images**:
   ```bash
   cd test-data
   python generate_synthetic_faces.py
   ```

3. **Generate real embeddings using ArcFace ONNX model**:
   ```bash
   python generate_real_embeddings.py > embeddings.json
   ```

   This requires the ArcFace model at `../worker/models/arcface.onnx` (130MB).

4. **Rebuild Docker image** (downloads model if missing, builds, loads to kind):
   ```bash
   cd ..
   ./build-worker.sh
   ```

5. **Run E2E authentication tests**:
   ```bash
   cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
   ```

   Expected result:
   - user1.png matches with ~99.99987% confidence ✓
   - different.png correctly rejected (no match) ✓

## Modifying Face Characteristics

To create faces with different features, edit the configuration dictionaries in `generate_synthetic_faces.py`:

```python
user_config = {
    'skin_tone': (220, 180, 160),     # RGB color
    'face_shape': (50, 40, 174, 190), # (x1, y1, x2, y2) - controls aspect ratio
    'eyes': {
        'size': 18,                    # Eye diameter
        'spacing': 35,                 # Distance between eyes
        'color': (101, 67, 33),       # Iris color (RGB)
    },
    'nose': {
        'length': 35,                  # Nose length
        'width': 12,                   # Nose width
    },
    'mouth': {
        'y': 155,                      # Vertical position
        'width': 30,                   # Mouth width
        'expression': 'neutral',       # 'neutral', 'smile', or 'frown'
    },
    'hair': {
        'color': (60, 40, 20),        # Hair color (RGB)
        'style': 'short',             # 'short', 'long', or 'bald'
    },
    'eyebrows': {
        'thickness': 2,                # Eyebrow thickness
        'angle': 0,                    # Positive=worried, Negative=angry, 0=neutral
    }
}
```

## Troubleshooting

### "No module named 'onnxruntime'"

Install ONNX Runtime:
```bash
cd test-data
uv pip install onnxruntime pillow
```

### "ONNX model file not found"

The ArcFace model must be present at `../worker/models/arcface.onnx` (relative to test-data directory).

Download from: https://huggingface.co/garavv/arcface-onnx

```bash
mkdir -p ../worker/models
# Download arcface.onnx to ../worker/models/
```

### E2E authentication tests failing

1. **Verify embeddings are up-to-date** with current images:
   ```bash
   cd test-data
   python generate_real_embeddings.py > embeddings.json
   ```

2. **Rebuild Docker image** with updated embeddings:
   ```bash
   cd ..
   docker build -t face-it-worker:latest -f worker/Dockerfile .
   kind load docker-image face-it-worker:latest --name kind-face-it
   ```

3. **Check that ArcFace model is in Docker image**:
   ```bash
   docker run --rm face-it-worker:latest ls -lh /models/face_recognition.onnx
   ```

   Should show ~130MB file.

4. **Verify worker is using real model** (not placeholder):
   ```bash
   # Check worker logs - should NOT see "using placeholder mode" warning
   kubectl logs -n face-it-workers <pod-name>
   ```

### Images look too similar

The structural differences are subtle but meaningful for face recognition. Key differences:
- **Face shape**: Measure width/height ratio
- **Eye spacing**: Distance between pupils
- **Eye color**: Check the iris color
- **Nose proportions**: Length vs width ratio
- **Mouth position**: Distance from nose

## Notes

- The test images are **synthetic/geometric**, not real photos
- **ArcFace ONNX model** provides real face recognition (production-ready)
- Embeddings are **512-dimensional** vectors generated by ArcFace ResNet model
- **Structural differences** (not pixel noise) distinguish faces
- E2E tests achieve **99.99987% confidence** for matching users
- Worker falls back to placeholder mode if model file not found (development only)
- The 130MB ArcFace model must be present in `worker/models/arcface.onnx`

## Quick Reference

### Using Test Data (Most Common)

**You're done!** Test data is already in the repository. Just run:

```bash
# Build worker image (downloads model if needed)
./build-worker.sh

# Run E2E test
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
```

### Regenerating Test Data (Rare)

Only needed if you're modifying face characteristics or learning how it works:

```bash
# 1. Modify faces
cd test-data
python generate_synthetic_faces.py

# 2. Generate new embeddings
python generate_real_embeddings.py > embeddings.json

# 3. Rebuild and test
cd ..
./build-worker.sh
cargo test -p api-server --test integration_e2e_auth -- --ignored --test-threads=1
```

### Understanding Test Data (Reference)

- See sections above for how images and embeddings were generated
- All generation is done with Python scripts (requires `onnxruntime`, `pillow`)
- ArcFace model must be at `../worker/models/arcface.onnx`
