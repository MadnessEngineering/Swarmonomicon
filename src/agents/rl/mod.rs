#[cfg(feature = "rl")]
use std::collections::HashMap;
#[cfg(feature = "rl")]
use rand::Rng;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

pub mod flappy;
pub mod model;
#[cfg(feature = "rl")]
pub mod viz;

/// Trait for states in reinforcement learning environments
#[cfg(feature = "rl")]
pub trait State: Clone + Eq + std::hash::Hash + Serialize + DeserializeOwned {
    fn to_features(&self) -> Vec<f64>;
}

/// Trait for actions in reinforcement learning environments
#[cfg(feature = "rl")]
pub trait Action: Clone + Eq + std::hash::Hash + Serialize + DeserializeOwned {
    fn to_index(&self) -> usize;
    fn from_index(index: usize) -> Option<Self>;
}

/// The environment interface that RL agents interact with
pub trait Environment {
    type S: State;
    type A: Action;

    /// Reset the environment to initial state
    fn reset(&mut self) -> Self::S;

    /// Take an action and return (new_state, reward, done)
    fn step(&mut self, action: &Self::A) -> (Self::S, f64, bool);

    /// Get the number of possible actions
    fn action_space_size(&self) -> usize;

    /// Get valid actions for current state
    fn valid_actions(&self, state: &Self::S) -> Vec<Self::A>;
}

/// Q-Learning agent implementation
#[cfg(feature = "rl")]
#[derive(Clone)]
pub struct QLearningAgent<S: State + Serialize + for<'de> Deserialize<'de>, A: Action + Serialize + for<'de> Deserialize<'de>> {
    q_table: HashMap<(S, A), f64>,
    pub metadata: model::QModelMetadata,
    state_size: usize,
    action_size: usize,
    learning_rate: f64,
    discount_factor: f64,
    epsilon: f64,
}

#[cfg(feature = "rl")]
impl<S: State + Serialize + for<'de> Deserialize<'de>, A: Action + Serialize + for<'de> Deserialize<'de>> QLearningAgent<S, A> {
    pub fn new(learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            metadata: model::QModelMetadata {
                version: model::MODEL_VERSION.to_string(),
                state_size: 0,
                action_size: 0,
                learning_rate,
                discount_factor,
                episodes_trained: 0,
                best_score: 0.0,
                epsilon,
                created_at: Some(chrono::Utc::now()),
                updated_at: Some(chrono::Utc::now()),
            },
            learning_rate,
            discount_factor,
            epsilon,
            state_size: 0,
            action_size: 0,
        }
    }

    /// Choose an action using epsilon-greedy policy
    pub fn choose_action(&mut self, state: &S, valid_actions: &[A]) -> A {
        // Update state_size and action_size if needed
        self.state_size = self.state_size.max(state.to_features().len());
        self.action_size = self.action_size.max(valid_actions.len());

        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() < self.epsilon {
            // Exploration: choose random action
            let idx = rng.gen_range(0..valid_actions.len());
            valid_actions[idx].clone()
        } else {
            // Exploitation: choose best action
            valid_actions
                .iter()
                .max_by(|a1, a2| {
                    let q1 = self.q_table.get(&(state.clone(), (*a1).clone())).unwrap_or(&0.0);
                    let q2 = self.q_table.get(&(state.clone(), (*a2).clone())).unwrap_or(&0.0);
                    q1.partial_cmp(q2).unwrap()
                })
                .unwrap()
                .clone()
        }
    }

    /// Update Q-value based on experience
    pub fn update(&mut self, state: &S, action: &A, reward: f64, next_state: &S) -> f64 {
        // Get valid actions for the next state (for a real implementation, you would pass these in)
        let valid_actions = vec![
            A::from_index(0).unwrap(),
            A::from_index(1).unwrap(),
        ];

        // First, find the maximum Q-value for the next state
        let next_max_q = valid_actions
            .iter()
            .map(|a| self.q_table.get(&(next_state.clone(), a.clone())).unwrap_or(&0.0))
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
            .max(0.0);

        // Then update the current Q-value
        let current_q = self.q_table.entry((state.clone(), action.clone())).or_insert(0.0);
        let old_q = *current_q;
        *current_q = (1.0 - self.learning_rate) * *current_q + 
                    self.learning_rate * (reward + self.discount_factor * next_max_q);
        
        // Return the new Q-value
        *current_q
    }

    /// Save the model to a file
    pub async fn save_model<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let mut model = model::QModel::new(
            self.state_size,
            self.action_size,
            self.learning_rate,
            self.discount_factor,
            self.epsilon,
        );
        
        // Update metadata
        model.metadata = self.metadata.clone();
        model.metadata.updated_at = Some(chrono::Utc::now());
        
        // Copy Q-table
        model.q_table = self.q_table.clone();
        
        // Save model to file
        model.save(path)
    }

    /// Load the model from a file
    pub async fn load_model<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let model = model::QModel::<S, A>::load(path)?;
        
        // Copy Q-table
        self.q_table = model.q_table;
        
        // Copy metadata
        self.metadata = model.metadata.clone();
        
        // Update agent parameters
        self.learning_rate = model.metadata.learning_rate;
        self.discount_factor = model.metadata.discount_factor;
        self.epsilon = model.metadata.epsilon;
        self.state_size = model.metadata.state_size;
        self.action_size = model.metadata.action_size;
        
        Ok(())
    }
    
    /// Save a checkpoint of the model
    pub async fn save_checkpoint<P: AsRef<Path>>(
        &self,
        base_path: P,
        episode: usize,
        is_best: bool,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let mut model = model::QModel::new(
            self.state_size,
            self.action_size,
            self.learning_rate,
            self.discount_factor,
            self.epsilon,
        );
        
        // Update metadata
        model.metadata = self.metadata.clone();
        model.metadata.episodes_trained = episode;
        model.metadata.updated_at = Some(chrono::Utc::now());
        
        // Copy Q-table
        model.q_table = self.q_table.clone();
        
        // Save checkpoint
        model.save_checkpoint(base_path, episode, is_best)
    }
    
    /// Load the latest checkpoint
    pub async fn load_latest_checkpoint<P: AsRef<Path>>(base_path: P) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        match model::QModel::<S, A>::load_latest_checkpoint(base_path)? {
            Some(model) => {
                let mut agent = Self::new(
                    model.metadata.learning_rate,
                    model.metadata.discount_factor,
                    model.metadata.epsilon,
                );
                
                // Copy Q-table and metadata
                agent.q_table = model.q_table;
                agent.metadata = model.metadata;
                agent.state_size = agent.metadata.state_size;
                agent.action_size = agent.metadata.action_size;
                
                Ok(Some(agent))
            },
            None => Ok(None),
        }
    }
    
    /// Clean up old checkpoint files
    pub fn clean_old_checkpoints<P: AsRef<Path>>(
        base_path: P,
        keep_latest: usize,
        keep_interval: Option<usize>,
    ) -> anyhow::Result<usize> {
        model::QModel::<S, A>::clean_old_checkpoints(base_path, keep_latest, keep_interval)
    }

    /// Get the configuration used for this agent
    pub fn get_config(&self) -> model::config::TrainingConfig {
        model::config::TrainingConfig {
            learning_rate: self.learning_rate,
            discount_factor: self.discount_factor,
            epsilon: self.epsilon,
            epsilon_decay: 0.999, // Default value
            min_epsilon: 0.01,    // Default value
            episodes: self.metadata.episodes_trained,
            visualize: false,
            checkpoint_freq: 100,
            checkpoint_path: "models".to_string(),
            save_metrics: true,
            metrics_path: "metrics".to_string(),
        }
    }

    /// Calculate the average Q-value for the current state
    pub fn calculate_avg_q_value(&self, state: &S) -> Option<f64> {
        let valid_actions = vec![
            A::from_index(0).unwrap(),
            A::from_index(1).unwrap(),
        ];
        
        if valid_actions.is_empty() {
            return None;
        }
        
        let sum: f64 = valid_actions.iter()
            .map(|a| self.q_table.get(&(state.clone(), a.clone())).unwrap_or(&0.0))
            .sum();
        
        Some(sum / valid_actions.len() as f64)
    }

    /// Decay the epsilon value based on the configuration
    pub fn decay_epsilon(&mut self, config: &model::config::TrainingConfig) {
        self.epsilon = (self.epsilon * config.epsilon_decay).max(config.min_epsilon);
        self.metadata.epsilon = self.epsilon;
    }
    
    /// Update agent metadata
    pub fn update_metadata(&mut self, 
                         episodes_trained: Option<usize>, 
                         best_score: Option<f64>,
                         epsilon: Option<f64>) {
        if let Some(episodes) = episodes_trained {
            self.metadata.episodes_trained = episodes;
        }
        
        if let Some(score) = best_score {
            self.metadata.best_score = score;
        }
        
        if let Some(eps) = epsilon {
            self.epsilon = eps;
            self.metadata.epsilon = eps;
        }
        
        self.metadata.updated_at = Some(chrono::Utc::now());
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use rand::Rng;
    use tempfile::tempdir;
    use std::fs;

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

    struct TestEnv {
        state: i32,
    }

    impl Environment for TestEnv {
        type S = TestState;
        type A = TestAction;

        fn reset(&mut self) -> Self::S {
            self.state = 0;
            TestState(self.state)
        }

        fn step(&mut self, action: &Self::A) -> (Self::S, f64, bool) {
            match action {
                TestAction::Up => self.state += 1,
                TestAction::Down => self.state -= 1,
            }
            
            let reward = if self.state == 5 { 1.0 } else { -0.1 };
            let done = self.state == 5 || self.state.abs() > 10;
            
            (TestState(self.state), reward, done)
        }

        fn action_space_size(&self) -> usize {
            2
        }

        fn valid_actions(&self, _state: &Self::S) -> Vec<Self::A> {
            vec![TestAction::Up, TestAction::Down]
        }
    }
    
    #[tokio::test]
    async fn test_agent_serialization() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_agent.json");
        
        let mut agent = QLearningAgent::<TestState, TestAction>::new(0.1, 0.95, 0.1);
        
        // Add some Q-values
        agent.q_table.insert((TestState(0), TestAction::Up), 0.5);
        agent.q_table.insert((TestState(1), TestAction::Down), -0.3);
        
        // Save the agent
        agent.save_model(&file_path).await.unwrap();
        
        // Load into a new agent
        let mut loaded_agent = QLearningAgent::<TestState, TestAction>::new(0.0, 0.0, 0.0);
        loaded_agent.load_model(&file_path).await.unwrap();
        
        // Check that Q-values match
        assert_eq!(agent.q_table, loaded_agent.q_table);
        
        // Check that parameters match
        assert_eq!(agent.learning_rate, loaded_agent.learning_rate);
        assert_eq!(agent.discount_factor, loaded_agent.discount_factor);
        assert_eq!(agent.epsilon, loaded_agent.epsilon);
    }
    
    #[tokio::test]
    async fn test_checkpoint_system() {
        let dir = tempdir().unwrap();
        let checkpoint_dir = dir.path();
        
        let mut agent = QLearningAgent::<TestState, TestAction>::new(0.1, 0.95, 0.1);
        
        // Add some Q-values for episode 10
        agent.q_table.insert((TestState(0), TestAction::Up), 0.1);
        
        // Save checkpoint for episode 10
        agent.save_checkpoint(checkpoint_dir, 10, false).await.unwrap();
        
        // Add more Q-values for episode 20
        agent.q_table.insert((TestState(1), TestAction::Down), 0.2);
        
        // Save checkpoint for episode 20 (as best model)
        agent.save_checkpoint(checkpoint_dir, 20, true).await.unwrap();
        
        // Add more Q-values for episode 30
        agent.q_table.insert((TestState(2), TestAction::Up), 0.3);
        
        // Save checkpoint for episode 30
        agent.save_checkpoint(checkpoint_dir, 30, false).await.unwrap();
        
        // Load the latest checkpoint
        let latest_agent = QLearningAgent::<TestState, TestAction>::load_latest_checkpoint(checkpoint_dir).await.unwrap();
        
        // Verify it's the latest one
        assert!(latest_agent.is_some());
        let latest = latest_agent.unwrap();
        assert_eq!(latest.metadata.episodes_trained, 30);
        
        // Make sure it has all Q-values
        assert_eq!(latest.q_table.len(), 3);
        assert_eq!(latest.q_table.get(&(TestState(0), TestAction::Up)), Some(&0.1));
        assert_eq!(latest.q_table.get(&(TestState(1), TestAction::Down)), Some(&0.2));
        assert_eq!(latest.q_table.get(&(TestState(2), TestAction::Up)), Some(&0.3));
        
        // Clean up old checkpoints
        let deleted = QLearningAgent::<TestState, TestAction>::clean_old_checkpoints(checkpoint_dir, 1, None).unwrap();
        assert_eq!(deleted, 2);
        
        // Load again, should still work
        let latest_again = QLearningAgent::<TestState, TestAction>::load_latest_checkpoint(checkpoint_dir).await.unwrap();
        assert!(latest_again.is_some());
    }

    #[test]
    fn test_qlearning() {
        let mut env = TestEnv { state: 0 };
        let mut agent = QLearningAgent::<TestState, TestAction>::new(0.1, 0.95, 0.1);
        
        // Run a simple episode
        let mut state = env.reset();
        let mut done = false;
        let mut total_reward = 0.0;
        
        while !done {
            // Choose action
            let valid_actions = env.valid_actions(&state);
            let action = agent.choose_action(&state, &valid_actions);
            
            // Take action
            let (new_state, reward, is_done) = env.step(&action);
            
            // Update Q-values
            agent.update(&state, &action, reward, &new_state);
            
            // Update state and done
            state = new_state;
            done = is_done;
            total_reward += reward;
        }
        
        // We should have some Q-values now
        assert!(!agent.q_table.is_empty());
    }
}
