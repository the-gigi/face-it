use crate::error::{WorkerError, WorkerResult};
use image::{DynamicImage, ImageBuffer, Rgb};
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;
use std::sync::Mutex;

/// Face recognition model wrapper
/// Loads ONNX models and generates embeddings from face images
pub struct FaceModel {
    session: Option<Mutex<Session>>,
    #[allow(dead_code)]
    model_path: String,
    input_size: (u32, u32),
    use_placeholder: bool,
}

impl FaceModel {
    /// Load a face recognition model from an ONNX file
    ///
    /// If the model file doesn't exist, falls back to placeholder mode
    /// for development/testing purposes
    pub fn load(path: &str) -> WorkerResult<Self> {
        let path_obj = Path::new(path);

        if !path_obj.exists() {
            tracing::warn!(
                "ONNX model file not found at {}, using placeholder mode. \
                 Set WORKER_USE_REAL_MODEL=false to suppress this warning.",
                path
            );

            return Ok(Self {
                session: None,
                model_path: path.to_string(),
                input_size: (112, 112), // Common face recognition input size
                use_placeholder: true,
            });
        }

        // Initialize ONNX Runtime
        match Session::builder() {
            Ok(builder) => {
                match builder.commit_from_file(path) {
                    Ok(session) => {
                        tracing::info!("Successfully loaded ONNX model from {}", path);

                        // Get input dimensions from model metadata
                        let input_size = Self::get_input_dimensions(&session)?;

                        Ok(Self {
                            session: Some(Mutex::new(session)),
                            model_path: path.to_string(),
                            input_size,
                            use_placeholder: false,
                        })
                    }
                    Err(e) => {
                        tracing::error!("Failed to load ONNX model: {}", e);
                        Err(WorkerError::Model(format!("Failed to load model: {}", e)))
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create ONNX session builder: {}", e);
                Err(WorkerError::Model(format!(
                    "Failed to create session: {}",
                    e
                )))
            }
        }
    }

    /// Extract input dimensions from the ONNX model
    fn get_input_dimensions(session: &Session) -> WorkerResult<(u32, u32)> {
        // Default to 112x112 if we can't determine from model
        // Most face recognition models use 112x112 or 160x160
        let _ = session; // Avoid unused warning
        Ok((112, 112))
    }

    /// Generate a face embedding from an image
    ///
    /// Takes raw image bytes, preprocesses them, and runs inference
    /// to produce a feature vector (embedding)
    pub fn generate_embedding(&self, image_data: &[u8]) -> WorkerResult<Vec<f32>> {
        if self.use_placeholder {
            return self.generate_placeholder_embedding(image_data);
        }

        // Decode image
        let img = image::load_from_memory(image_data)
            .map_err(|e| WorkerError::Image(format!("Failed to decode image: {}", e)))?;

        // Preprocess image
        let preprocessed = self.preprocess_image(img)?;

        // Run inference
        self.run_inference(&preprocessed)
    }

    /// Preprocess image for model input
    ///
    /// Steps:
    /// 1. Resize to model input size (112x112 for ArcFace)
    /// 2. Convert to RGB
    /// 3. Normalize pixel values: (pixel - 127.5) / 128.0
    /// 4. Convert to NHWC format (batch, height, width, channels) for ArcFace model
    fn preprocess_image(&self, img: DynamicImage) -> WorkerResult<Array4<f32>> {
        // Resize to model input dimensions (112x112 for ArcFace)
        let img = img.resize_exact(
            self.input_size.0,
            self.input_size.1,
            image::imageops::FilterType::Triangle,
        );

        // Convert to RGB
        let rgb_img: ImageBuffer<Rgb<u8>, Vec<u8>> = img.to_rgb8();

        // Get dimensions
        let (width, height) = rgb_img.dimensions();

        // Create array in NHWC format: (1, height, width, 3)
        // ArcFace model expects NHWC (channels last) format
        let mut array = Array4::<f32>::zeros((1, height as usize, width as usize, 3));

        // Fill array with normalized pixel values
        // ArcFace normalization: (pixel - 127.5) / 128.0
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb_img.get_pixel(x, y);

                array[[0, y as usize, x as usize, 0]] = (pixel[0] as f32 - 127.5) / 128.0; // R
                array[[0, y as usize, x as usize, 1]] = (pixel[1] as f32 - 127.5) / 128.0; // G
                array[[0, y as usize, x as usize, 2]] = (pixel[2] as f32 - 127.5) / 128.0;
                // B
            }
        }

        Ok(array)
    }

    /// Run inference using the ONNX model
    fn run_inference(&self, input: &Array4<f32>) -> WorkerResult<Vec<f32>> {
        let session_mutex = self
            .session
            .as_ref()
            .ok_or_else(|| WorkerError::Model("Model not loaded".to_string()))?;

        // Lock the mutex to get mutable access to the session
        let mut session = session_mutex
            .lock()
            .map_err(|e| WorkerError::Model(format!("Failed to lock session mutex: {}", e)))?;

        // Create TensorRef using the (shape, &[T]) format which is explicitly supported
        // Get raw slice from the array (must be contiguous)
        let input_slice = input
            .as_slice()
            .ok_or_else(|| WorkerError::Model("Input array not contiguous".to_string()))?;

        // Get shape as a slice
        let shape = input.shape();

        // Create tensor from shape and slice tuple
        let input_tensor = TensorRef::from_array_view((shape, input_slice))
            .map_err(|e| WorkerError::Model(format!("Failed to create tensor: {}", e)))?;

        // Run inference using the ort 2.0 API
        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|e| WorkerError::Model(format!("Inference failed: {}", e)))?;

        // Extract first output (face embedding vector)
        // SessionOutputs is an array-like structure, access by index
        let output_value = &outputs[0];

        // Extract as tensor - returns (shape, data_slice)
        let (_shape, output_data) = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| WorkerError::Model(format!("Failed to extract output tensor: {}", e)))?;

        // Convert slice to Vec
        let embedding_vec: Vec<f32> = output_data.to_vec();

        Ok(embedding_vec)
    }

    /// Generate a placeholder embedding for testing/development
    ///
    /// Returns a deterministic embedding based on image content
    /// This is NOT suitable for production use - only for testing
    fn generate_placeholder_embedding(&self, image_data: &[u8]) -> WorkerResult<Vec<f32>> {
        tracing::debug!("Generating placeholder embedding (not a real face embedding)");

        // Generate a 512-dimensional embedding based on image content
        // This ensures similar images produce similar embeddings for testing
        let mut embedding = vec![0.0f32; 512];

        // Hash image bytes into embedding vector
        let chunk_size = (image_data.len() / 512).max(1);
        for (i, chunk) in image_data.chunks(chunk_size).enumerate() {
            if i >= 512 {
                break;
            }
            let sum: u32 = chunk.iter().map(|&b| b as u32).sum();
            // Normalize to [-1, 1] range
            embedding[i] = ((sum % 2000) as f32 / 1000.0) - 1.0;
        }

        // Normalize the embedding to unit length
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        Ok(embedding)
    }

    /// Check if model is in placeholder mode
    #[allow(dead_code)]
    pub fn is_placeholder(&self) -> bool {
        self.use_placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_model_load_placeholder_mode() {
        // Non-existent path should trigger placeholder mode
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();
        assert!(model.use_placeholder);
        assert_eq!(model.input_size, (112, 112));
    }

    #[test]
    fn test_generate_placeholder_embedding() {
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();
        let embedding = model.generate_embedding(b"fake image data").unwrap();

        assert_eq!(embedding.len(), 512);

        // Embedding should be normalized (unit length)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 0.001,
            "Embedding should be normalized to unit length"
        );

        // Same input should produce same embedding (deterministic)
        let embedding2 = model.generate_embedding(b"fake image data").unwrap();
        assert_eq!(embedding, embedding2);
    }

    #[test]
    fn test_preprocess_image() {
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();

        // Create a simple 2x2 RGB image
        let img = DynamicImage::ImageRgb8(ImageBuffer::from_fn(2, 2, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255u8, 0u8, 0u8]) // Red
            } else {
                Rgb([0u8, 255u8, 0u8]) // Green
            }
        }));

        let preprocessed = model.preprocess_image(img).unwrap();

        // Check shape: (1, 112, 112, 3) - NHWC format for ArcFace
        assert_eq!(preprocessed.dim(), (1, 112, 112, 3));

        // Check values are normalized to ArcFace range: (pixel - 127.5) / 128.0
        // Range should be approximately [-1, 1]
        let max_val = preprocessed
            .iter()
            .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = preprocessed.iter().fold(f32::INFINITY, |a, &b| a.min(b));

        assert!(max_val <= 1.0);
        assert!(min_val >= -1.0);
    }

    #[test]
    fn test_generate_embedding_from_simple_image() {
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();

        // Create a simple image in memory
        let img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([128u8, 128u8, 128u8]));

        let mut img_bytes = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut img_bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        let embedding = model.generate_embedding(&img_bytes).unwrap();
        assert_eq!(embedding.len(), 512);
    }

    #[test]
    fn test_generate_embedding_invalid_image() {
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();

        // Since we're in placeholder mode, it should still return an embedding
        // even with invalid image data
        let result = model.generate_embedding(b"not an image");

        // In placeholder mode, image decoding is bypassed
        assert!(result.is_ok());
    }

    #[test]
    fn test_different_images_different_embeddings_in_placeholder() {
        let model = FaceModel::load("/nonexistent/model.onnx").unwrap();

        let emb1 = model.generate_embedding(b"image1").unwrap();
        let emb2 = model.generate_embedding(b"image2").unwrap();

        // In placeholder mode, different images produce different embeddings
        // based on content hashing
        assert_ne!(emb1, emb2);
        assert_eq!(emb1.len(), 512);
        assert_eq!(emb2.len(), 512);
    }
}
