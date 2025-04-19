use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod config;
pub use config::{TrainingConfig, TrainingHistory, TrainingMetrics};

pub const MODEL_VERSION: &str = "1.0.0";
const CHECKPOINT_PREFIX: &str = "checkpoint_";
const BEST_MODEL_FILENAME: &str = "best_model.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QModelMetadata {
    pub version: String,
    pub state_size: usize,
    pub action_size: usize,
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub episodes_trained: usize,
    pub best_score: f64,
    pub epsilon: f64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct QModel<S, A> {
    pub metadata: QModelMetadata,
    pub q_table: HashMap<(S, A), f64>,
}

impl<S, A> QModel<S, A>
where
    S: Serialize + for<'de> Deserialize<'de> + Eq + Hash + Clone,
    A: Serialize + for<'de> Deserialize<'de> + Eq + Hash + Clone,
{
    pub fn new(
        state_size: usize,
        action_size: usize,
        learning_rate: f64,
        discount_factor: f64,
        epsilon: f64,
    ) -> Self {
        Self {
            metadata: QModelMetadata {
                version: MODEL_VERSION.to_string(),
                state_size,
                action_size,
                learning_rate,
                discount_factor,
                episodes_trained: 0,
                best_score: 0.0,
                epsilon,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            },
            q_table: HashMap::new(),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
    where
        S: Serialize,
        A: Serialize,
    {
        // Update metadata before saving
        let mut serializable = SerializableQModel::from(self);
        serializable.metadata.updated_at = Some(Utc::now());

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&serializable)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        S: for<'de> Deserialize<'de>,
        A: for<'de> Deserialize<'de>,
    {
        let json = fs::read_to_string(path)?;
        let serializable: SerializableQModel<S, A> = serde_json::from_str(&json)?;

        // Check version
        if serializable.metadata.version != MODEL_VERSION {
            println!(
                "Warning: Loading model with version {} (current version is {})",
                serializable.metadata.version, MODEL_VERSION
            );
            // In a real system, we might implement version migration here
        }

        Ok(serializable.into())
    }

    pub fn save_checkpoint<P: AsRef<Path>>(
        &self,
        base_path: P,
        episode: usize,
        is_best: bool,
    ) -> Result<PathBuf, Box<dyn std::error::Error>>
    where
        S: Serialize,
        A: Serialize,
    {
        // Create checkpoint directory
        let base_dir = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)?;

        // Save regular checkpoint
        let checkpoint_path = base_dir.join(format!("{}{:06}.json", CHECKPOINT_PREFIX, episode));
        self.save(&checkpoint_path)?;

        // Save as best model if needed
        if is_best {
            let best_model_path = base_dir.join(BEST_MODEL_FILENAME);
            self.save(&best_model_path)?;
        }

        Ok(checkpoint_path)
    }

    pub fn load_latest_checkpoint<P: AsRef<Path>>(base_path: P) -> Result<Option<Self>>
    where
        S: for<'de> Deserialize<'de>,
        A: for<'de> Deserialize<'de>,
    {
        let base_dir = base_path.as_ref().to_path_buf();
        if !base_dir.exists() {
            return Ok(None);
        }

        // Search for all checkpoint files
        let mut checkpoints = Vec::new();
        for entry in fs::read_dir(&base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if filename_str.starts_with(CHECKPOINT_PREFIX)
                            && filename_str.ends_with(".json")
                        {
                            checkpoints.push(path);
                        }
                    }
                }
            }
        }

        // Find latest checkpoint
        if let Some(latest) = checkpoints.iter().max_by(|a, b| {
            a.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .cmp(b.file_name().unwrap().to_str().unwrap())
        }) {
            return Ok(Some(Self::load(latest)?));
        }

        // If no checkpoint found, try to load the best model
        let best_model_path = base_dir.join(BEST_MODEL_FILENAME);
        if best_model_path.exists() {
            return Ok(Some(Self::load(best_model_path)?));
        }

        Ok(None)
    }

    pub fn update_metadata(
        &mut self,
        episodes_trained: Option<usize>,
        best_score: Option<f64>,
        epsilon: Option<f64>,
    ) {
        if let Some(episodes) = episodes_trained {
            self.metadata.episodes_trained = episodes;
        }

        if let Some(score) = best_score {
            self.metadata.best_score = score;
        }

        if let Some(eps) = epsilon {
            self.metadata.epsilon = eps;
        }

        self.metadata.updated_at = Some(Utc::now());
    }

    pub fn clean_old_checkpoints<P: AsRef<Path>>(
        base_path: P,
        keep_latest: usize,
        keep_interval: Option<usize>,
    ) -> Result<usize> {
        let base_dir = base_path.as_ref().to_path_buf();
        if !base_dir.exists() {
            return Ok(0);
        }

        // Find all checkpoint files
        let mut checkpoints = Vec::new();
        for entry in fs::read_dir(&base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if filename_str.starts_with(CHECKPOINT_PREFIX)
                            && filename_str.ends_with(".json")
                        {
                            checkpoints.push(path);
                        }
                    }
                }
            }
        }

        // Sort checkpoints by name (which contains the episode number)
        checkpoints.sort_by(|a, b| {
            a.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .cmp(b.file_name().unwrap().to_str().unwrap())
        });

        // Decide which ones to keep
        let mut to_delete = Vec::new();
        let num_checkpoints = checkpoints.len();

        if num_checkpoints <= keep_latest {
            return Ok(0);
        }

        for (i, checkpoint) in checkpoints.iter().enumerate() {
            // Keep the latest 'keep_latest' checkpoints
            if i >= num_checkpoints - keep_latest {
                continue;
            }

            // Keep checkpoints at regular intervals if specified
            if let Some(interval) = keep_interval {
                // Extract episode number from filename
                if let Some(filename) = checkpoint.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if filename_str.starts_with(CHECKPOINT_PREFIX)
                            && filename_str.ends_with(".json")
                        {
                            let episode_part =
                                &filename_str[CHECKPOINT_PREFIX.len()..filename_str.len() - 5];
                            if let Ok(episode) = episode_part.parse::<usize>() {
                                if episode % interval == 0 {
                                    continue; // Keep this interval checkpoint
                                }
                            }
                        }
                    }
                }
            }

            to_delete.push(checkpoint);
        }

        // Delete the checkpoints
        let mut deleted = 0;
        for checkpoint in to_delete {
            if fs::remove_file(&checkpoint).is_ok() {
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(
    bound = "S: Serialize + for<'a> Deserialize<'a> + Eq + Hash, A: Serialize + for<'a> Deserialize<'a> + Eq + Hash"
)]
struct SerializableQModel<S, A> {
    metadata: QModelMetadata,
    q_table: HashMap<(S, A), f64>,
}

impl<S, A> From<&QModel<S, A>> for SerializableQModel<S, A>
where
    S: Clone,
    A: Clone,
{
    fn from(model: &QModel<S, A>) -> Self {
        Self {
            metadata: model.metadata.clone(),
            q_table: model.q_table.clone(),
        }
    }
}

impl<S, A> From<SerializableQModel<S, A>> for QModel<S, A> {
    fn from(serializable: SerializableQModel<S, A>) -> Self {
        Self {
            metadata: serializable.metadata,
            q_table: serializable.q_table,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_model_creation() {
        let model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99, 0.1);
        assert_eq!(model.metadata.state_size, 10);
        assert_eq!(model.metadata.action_size, 4);
        assert_eq!(model.metadata.learning_rate, 0.1);
        assert_eq!(model.metadata.discount_factor, 0.99);
        assert_eq!(model.metadata.epsilon, 0.1);
        assert_eq!(model.metadata.episodes_trained, 0);
        assert_eq!(model.metadata.best_score, 0.0);
        assert!(model.q_table.is_empty());
    }

    #[test]
    fn test_model_serialization() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("model.json");

        let mut model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99, 0.1);
        model.q_table.insert((1, 2), 0.5);
        model.save(&file_path).unwrap();

        let loaded_model = QModel::<u32, u32>::load(&file_path).unwrap();
        assert_eq!(loaded_model.metadata.state_size, model.metadata.state_size);
        assert_eq!(
            loaded_model.metadata.action_size,
            model.metadata.action_size
        );
        assert_eq!(
            loaded_model.metadata.learning_rate,
            model.metadata.learning_rate
        );
        assert_eq!(
            loaded_model.metadata.discount_factor,
            model.metadata.discount_factor
        );
        assert_eq!(loaded_model.metadata.epsilon, model.metadata.epsilon);
        assert_eq!(loaded_model.q_table, model.q_table);
    }

    #[test]
    fn test_checkpoint_system() {
        let dir = tempdir().unwrap();
        let base_path = dir.path();

        let model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99, 0.1);

        // Save checkpoints
        model.save_checkpoint(base_path, 100, false).unwrap();
        model.save_checkpoint(base_path, 200, true).unwrap();
        model.save_checkpoint(base_path, 300, false).unwrap();

        // Load latest checkpoint
        let loaded = QModel::<u32, u32>::load_latest_checkpoint(base_path).unwrap();
        assert!(loaded.is_some());
        let loaded_model = loaded.unwrap();
        assert_eq!(loaded_model.metadata.state_size, model.metadata.state_size);

        // Test cleaning old checkpoints
        let deleted = QModel::<u32, u32>::clean_old_checkpoints(base_path, 1, None).unwrap();
        assert_eq!(deleted, 2);
    }
}
