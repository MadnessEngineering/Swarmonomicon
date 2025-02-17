use serde::{Serialize, Deserialize};
use std::marker::PhantomData;
use anyhow::Result;
use std::path::Path;

pub const MODEL_VERSION: &str = "1.0.0";

#[derive(Debug, Serialize, Deserialize)]
pub struct QModelMetadata {
    pub version: String,
    pub state_size: usize,
    pub action_size: usize,
    pub learning_rate: f32,
    pub discount_factor: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QModel<S, A> {
    pub metadata: QModelMetadata,
    pub q_table: Vec<Vec<f32>>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> QModel<S, A> {
    pub fn new(state_size: usize, action_size: usize, learning_rate: f32, discount_factor: f32) -> Self {
        Self {
            metadata: QModelMetadata {
                version: MODEL_VERSION.to_string(),
                state_size,
                action_size,
                learning_rate,
                discount_factor,
            },
            q_table: vec![vec![0.0; action_size]; state_size],
            _phantom: PhantomData,
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let model = serde_json::from_str(&json)?;
        Ok(model)
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_model_creation() {
        let model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99);
        assert_eq!(model.metadata.state_size, 10);
        assert_eq!(model.metadata.action_size, 4);
        assert_eq!(model.metadata.learning_rate, 0.1);
        assert_eq!(model.metadata.discount_factor, 0.99);
        assert_eq!(model.q_table.len(), 10);
        assert_eq!(model.q_table[0].len(), 4);
    }

    #[test]
    fn test_model_serialization() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("model.json");
        
        let model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99);
        model.save(&file_path).unwrap();
        
        let loaded_model = QModel::<u32, u32>::load(&file_path).unwrap();
        assert_eq!(loaded_model.metadata.state_size, model.metadata.state_size);
        assert_eq!(loaded_model.metadata.action_size, model.metadata.action_size);
        assert_eq!(loaded_model.metadata.learning_rate, model.metadata.learning_rate);
        assert_eq!(loaded_model.metadata.discount_factor, model.metadata.discount_factor);
        assert_eq!(loaded_model.q_table, model.q_table);
    }
} 
