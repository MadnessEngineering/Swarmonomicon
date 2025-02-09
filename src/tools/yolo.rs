#[cfg(feature = "yolo")]
mod yolo {
    use std::path::Path;
    use std::collections::HashMap;
    use async_trait::async_trait;
    use crate::tools::ToolExecutor;
    use anyhow::{Result, anyhow};
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Detection {
        pub class: String,
        pub confidence: f32,
        pub bbox: [f32; 4], // [x, y, width, height]
    }

    pub struct YoloTool {
        model_path: String,
        config_path: String,
    }

    impl YoloTool {
        pub fn new() -> Self {
            Self {
                model_path: "models/yolov3.weights".to_string(),
                config_path: "models/yolov3.cfg".to_string(),
            }
        }

        pub fn detect_objects<P: AsRef<Path>>(&self, image_path: P) -> Result<Vec<Detection>> {
            // Check if image exists
            if !image_path.as_ref().exists() {
                return Err(anyhow!("Image file does not exist"));
            }

            // Check if model files exist
            if !Path::new(&self.model_path).exists() || !Path::new(&self.config_path).exists() {
                return Err(anyhow!("YOLO model files not found"));
            }

            // Load OpenCV DNN module
            let net = opencv::dnn::read_net_from_darknet(
                &self.config_path,
                &self.model_path,
            ).map_err(|e| anyhow!("Failed to load YOLO model: {}", e))?;

            // Load and preprocess image
            let image = opencv::imgcodecs::imread(
                image_path.as_ref().to_str().unwrap(),
                opencv::imgcodecs::IMREAD_COLOR,
            ).map_err(|e| anyhow!("Failed to load image: {}", e))?;

            // Create blob from image
            let blob = opencv::dnn::blob_from_image(
                &image,
                1.0/255.0,
                opencv::core::Size::new(416, 416),
                opencv::core::Vector::from_slice(&[0f64, 0f64, 0f64]),
                true,
                false,
                opencv::core::CV_32F,
            ).map_err(|e| anyhow!("Failed to create blob: {}", e))?;

            // Set input and forward pass
            net.set_input(&blob, "", 1.0, opencv::core::Vector::default())
                .map_err(|e| anyhow!("Failed to set network input: {}", e))?;

            let mut detections = Vec::new();
            let output_layers = net.get_unconnected_out_layers()
                .map_err(|e| anyhow!("Failed to get output layers: {}", e))?;

            for layer in output_layers.iter() {
                let output = net.forward("", layer)
                    .map_err(|e| anyhow!("Failed to perform forward pass: {}", e))?;

                // Process detections
                for i in 0..output.rows() {
                    let confidence = output.at_2d::<f32>(i as i32, 4)
                        .map_err(|e| anyhow!("Failed to get confidence: {}", e))?;

                    if confidence > 0.5 {
                        let x = output.at_2d::<f32>(i as i32, 0)
                            .map_err(|e| anyhow!("Failed to get x: {}", e))?;
                        let y = output.at_2d::<f32>(i as i32, 1)
                            .map_err(|e| anyhow!("Failed to get y: {}", e))?;
                        let w = output.at_2d::<f32>(i as i32, 2)
                            .map_err(|e| anyhow!("Failed to get width: {}", e))?;
                        let h = output.at_2d::<f32>(i as i32, 3)
                            .map_err(|e| anyhow!("Failed to get height: {}", e))?;

                        detections.push(Detection {
                            class: "object".to_string(), // You can add class labels if needed
                            confidence,
                            bbox: [x, y, w, h],
                        });
                    }
                }
            }

            Ok(detections)
        }
    }

    #[async_trait]
    impl ToolExecutor for YoloTool {
        async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
            let image_path = params.get("image")
                .ok_or_else(|| anyhow!("Missing image parameter"))?;

            let detections = self.detect_objects(image_path)?;
            
            // Convert detections to JSON string
            serde_json::to_string(&detections)
                .map_err(|e| anyhow!("Failed to serialize detections: {}", e))
        }
    }
} 
