#[cfg(feature = "yolo")]
mod yolo_test {
    use super::*;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_yolo_detection() -> Result<(), Box<dyn std::error::Error>> {
        // Create test image path
        let test_image = PathBuf::from("test_data/test_image.jpg");
        
        // Ensure test directory exists
        std::fs::create_dir_all("test_data")?;
        
        // Create a simple test image if it doesn't exist
        if !test_image.exists() {
            // Create a simple test image using image crate
            let imgbuf = image::ImageBuffer::new(100, 100);
            imgbuf.save(&test_image)?;
        }

        // Initialize yolo tool
        let yolo = YoloTool::new();
        
        // Run detection
        let result = yolo.detect_objects(&test_image)?;
        
        // Basic validation
        assert!(!result.is_empty(), "Should detect at least one object");
        
        // Cleanup
        std::fs::remove_file(test_image)?;
        std::fs::remove_dir("test_data")?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_yolo_invalid_image() -> Result<(), Box<dyn std::error::Error>> {
        let yolo = YoloTool::new();
        let invalid_path = PathBuf::from("nonexistent.jpg");
        
        let result = yolo.detect_objects(&invalid_path);
        assert!(result.is_err(), "Should return error for invalid image");
        
        Ok(())
    }
} 
