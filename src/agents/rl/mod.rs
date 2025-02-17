#[cfg(feature = "rl")]
use std::collections::HashMap;
#[cfg(feature = "rl")]
use rand::Rng;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

pub mod flappy;
pub mod model;

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
pub struct QLearningAgent<S: State + Serialize + DeserializeOwned, A: Action + Serialize + DeserializeOwned> {
    q_table: HashMap<(S, A), f64>,
    learning_rate: f64,
    discount_factor: f64,
    epsilon: f64,
}

#[cfg(feature = "rl")]
impl<S: State + Serialize + DeserializeOwned, A: Action + Serialize + DeserializeOwned> QLearningAgent<S, A> {
    pub fn new(learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate,
            discount_factor,
            epsilon,
        }
    }

    /// Choose an action using epsilon-greedy policy
    pub fn choose_action(&self, state: &S, valid_actions: &[A]) -> A {
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
    pub fn learn(&mut self, state: S, action: A, reward: f64, next_state: &S, next_valid_actions: &[A]) {
        // First, find the maximum Q-value for the next state
        let next_max_q = next_valid_actions
            .iter()
            .map(|a| self.q_table.get(&(next_state.clone(), a.clone())).unwrap_or(&0.0))
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
            .max(0.0);

        // Then update the current Q-value
        let current_q = self.q_table.entry((state, action)).or_insert(0.0);
        *current_q = (1.0 - self.learning_rate) * *current_q + 
                    self.learning_rate * (reward + self.discount_factor * next_max_q);
    }

    /// Save the model to a file
    pub fn save_model<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let model = model::QModel {
            metadata: model::QModelMetadata {
                version: model::MODEL_VERSION.to_string(),
                episodes_trained: 0,
                best_score: 0.0,
                learning_rate: self.learning_rate,
                discount_factor: self.discount_factor,
                epsilon: self.epsilon,
            },
            q_table: self.q_table.clone(),
        };
        model.save(path)
    }

    /// Load the model from a file
    pub fn load_model<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let model = model::QModel::<S, A>::load(path)?;
        Ok(Self {
            q_table: model.q_table,
            learning_rate: model.metadata.learning_rate,
            discount_factor: model.metadata.discount_factor,
            epsilon: model.metadata.epsilon,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

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

    #[test]
    fn test_qlearning() {
        let mut agent = QLearningAgent::new(0.1, 0.95, 0.1);
        let mut env = TestEnv { state: 0 };

        // Run one episode
        let mut state = env.reset();
        let mut total_reward = 0.0;

        for _ in 0..100 {
            let valid_actions = env.valid_actions(&state);
            let action = agent.choose_action(&state, &valid_actions);
            let (next_state, reward, done) = env.step(&action);

            total_reward += reward;

            let next_valid_actions = env.valid_actions(&next_state);
            agent.learn(state.clone(), action, reward, &next_state, &next_valid_actions);

            if done {
                break;
            }
            state = next_state;
        }

        assert!(total_reward != 0.0, "Agent should have received some rewards");
    }
}
