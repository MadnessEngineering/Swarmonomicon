#[cfg(feature = "rl")]
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::de::DeserializeOwned;
use super::{State, Action};
use std::hash::Hash;

pub const MODEL_VERSION: &str = "0.1.0";

#[derive(Debug, Serialize, Deserialize)]
pub struct QModelMetadata {
    pub version: String,
    pub episodes_trained: u32,
    pub best_score: f64,
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub epsilon: f64,
}

impl Default for QModelMetadata {
    fn default() -> Self {
        Self {
            version: MODEL_VERSION.to_string(),
            episodes_trained: 0,
            best_score: 0.0,
            learning_rate: 0.1,
            discount_factor: 0.99,
            epsilon: 0.1,
        }
    }
}

#[derive(Debug)]
pub struct QModel<S, A> {
    pub metadata: QModelMetadata,
    pub q_table: HashMap<(S, A), f64>,
}

impl<S, A> QModel<S, A>
where
    S: State + Serialize + DeserializeOwned + Eq + Hash,
    A: Action + Serialize + DeserializeOwned + Eq + Hash,
{
    pub fn new(learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        Self {
            metadata: QModelMetadata {
                learning_rate,
                discount_factor,
                epsilon,
                ..Default::default()
            },
            q_table: HashMap::new(),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
    where
        S: Serialize,
        A: Serialize,
    {
        #[derive(Serialize)]
        struct SerializedModel<'a, S, A> {
            metadata: &'a QModelMetadata,
            q_table: &'a HashMap<(S, A), f64>,
        }

        let serialized = SerializedModel {
            metadata: &self.metadata,
            q_table: &self.q_table,
        };

        let json = serde_json::to_string_pretty(&serialized)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        S: DeserializeOwned,
        A: DeserializeOwned,
    {
        #[derive(Deserialize)]
        struct SerializedModel<S, A>
        where
            S: Eq + Hash,
            A: Eq + Hash,
        {
            metadata: QModelMetadata,
            q_table: HashMap<(S, A), f64>,
        }

        let json = fs::read_to_string(path)?;
        let serialized: SerializedModel<S, A> = serde_json::from_str(&json)?;

        Ok(Self {
            metadata: serialized.metadata,
            q_table: serialized.q_table,
        })
    }

    pub fn update_training_stats(&mut self, episodes: i32, score: i32) {
        self.metadata.episodes_trained = episodes as u32;
        if score as f64 > self.metadata.best_score {
            self.metadata.best_score = score as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    struct TestState(i32);

    impl State for TestState {
        fn to_features(&self) -> Vec<f64> {
            vec![self.0 as f64]
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    enum TestAction {
        Up,
        Down,
    }

    impl Action for TestAction {
        fn to_index(&self) -> usize {
            match self {
                TestAction::Up => 0,
                TestAction::Down => 1,
            }
        }

        fn from_index(index: usize) -> Option<Self> {
            match index {
                0 => Some(TestAction::Up),
                1 => Some(TestAction::Down),
                _ => None,
            }
        }
    }

    #[test]
    fn test_model_serialization() {
        let mut model = QModel::<TestState, TestAction>::new(0.1, 0.99, 0.1);
        model.q_table.insert((TestState(1), TestAction::Up), 0.5);
        model.q_table.insert((TestState(1), TestAction::Down), -0.5);
        model.update_training_stats(100, 10);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_model.json");

        // Test saving
        model.save(&file_path).unwrap();
        assert!(file_path.exists());

        // Test loading
        let loaded_model = QModel::<TestState, TestAction>::load(&file_path).unwrap();
        assert_eq!(loaded_model.metadata.episodes_trained, 100);
        assert_eq!(loaded_model.metadata.best_score, 10.0);
        assert_eq!(loaded_model.q_table.len(), 2);
        assert_eq!(loaded_model.q_table.get(&(TestState(1), TestAction::Up)), Some(&0.5));
    }
} 
