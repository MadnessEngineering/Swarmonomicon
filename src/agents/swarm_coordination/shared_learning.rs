/// Shared Q-Learning for Multi-Agent RL
///
/// Multiple agents share and update a common Q-table, enabling:
/// - Collective learning from distributed experiences
/// - Faster convergence through shared knowledge
/// - Emergent coordinated behaviors

use std::collections::HashMap;
use std::hash::Hash;
use mongodb::{Client, Collection};
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Shared state-action pair for MongoDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedQEntry<S, A> {
    pub state: S,
    pub action: A,
    pub q_value: f64,
    pub update_count: usize,
    pub last_updated_by: String,
    pub last_updated_at: i64,
}

/// Generic shared state trait
pub trait SharedState: Clone + Eq + Hash + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {}
impl<T> SharedState for T where T: Clone + Eq + Hash + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {}

/// Generic shared action trait
pub trait SharedAction: Clone + Eq + Hash + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {}
impl<T> SharedAction for T where T: Clone + Eq + Hash + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {}

/// Shared Q-Learning with MongoDB persistence
pub struct SharedQLearning {
    // In-memory cache of Q-values
    q_table: HashMap<String, f64>, // key: hash(state, action)
    learning_rate: f64,
    discount_factor: f64,
    // MongoDB for persistence (type-erased for flexibility)
    collection: Collection<mongodb::bson::Document>,
}

impl SharedQLearning {
    pub async fn new(mongo_client: Client) -> Result<Self> {
        let db = mongo_client.database("swarmonomicon");
        let collection = db.collection("shared_q_learning");

        // Create indexes
        use mongodb::IndexModel;
        use mongodb::bson::doc;

        let indexes = vec![
            IndexModel::builder()
                .keys(doc! { "state_hash": 1, "action_hash": 1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "last_updated_at": -1 })
                .build(),
        ];

        collection.create_indexes(indexes, None).await?;

        Ok(Self {
            q_table: HashMap::new(),
            learning_rate: 0.1,
            discount_factor: 0.95,
            collection,
        })
    }

    /// Update Q-value from agent experience
    pub async fn update<S, A>(
        &mut self,
        agent_id: &str,
        state: &S,
        action: &A,
        reward: f64,
        next_state: &S,
    ) -> Result<f64>
    where
        S: SharedState,
        A: SharedAction,
    {
        let key = Self::hash_state_action(state, action);

        // Get current Q-value
        let current_q = *self.q_table.get(&key).unwrap_or(&0.0);

        // Estimate max Q for next state (simplified - would need valid actions)
        let next_max_q = 0.0; // Placeholder

        // Q-learning update
        let new_q = (1.0 - self.learning_rate) * current_q
            + self.learning_rate * (reward + self.discount_factor * next_max_q);

        // Update in-memory
        self.q_table.insert(key.clone(), new_q);

        // Persist to MongoDB (async, non-blocking)
        self.persist_q_value(agent_id, &key, new_q).await?;

        Ok(new_q)
    }

    /// Get Q-value for state-action pair
    pub fn get_q_value<S, A>(&self, state: &S, action: &A) -> f64
    where
        S: SharedState,
        A: SharedAction,
    {
        let key = Self::hash_state_action(state, action);
        *self.q_table.get(&key).unwrap_or(&0.0)
    }

    /// Choose best action for state (exploitation)
    pub fn best_action<S, A>(&self, state: &S, valid_actions: &[A]) -> Option<A>
    where
        S: SharedState,
        A: SharedAction,
    {
        let mut best_action = None;
        let mut best_q = f64::NEG_INFINITY;

        for action in valid_actions {
            let q = self.get_q_value(state, action);
            if q > best_q {
                best_q = q;
                best_action = Some(action.clone());
            }
        }

        best_action
    }

    /// Train from collective experience
    pub async fn train_from_collective_experience(&mut self) -> Result<()> {
        // Load all Q-values from MongoDB into memory
        use mongodb::bson::doc;
        use futures_util::TryStreamExt;

        let mut cursor = self.collection.find(None, None).await?;

        while let Some(doc) = cursor.try_next().await? {
            if let (Some(key), Some(q_value)) = (
                doc.get_str("key").ok(),
                doc.get_f64("q_value").ok()
            ) {
                self.q_table.insert(key.to_string(), q_value);
            }
        }

        Ok(())
    }

    /// Hash state-action pair for storage key
    fn hash_state_action<S, A>(state: &S, action: &A) -> String
    where
        S: std::fmt::Debug,
        A: std::fmt::Debug,
    {
        format!("{:?}_{:?}", state, action)
    }

    /// Persist Q-value to MongoDB
    async fn persist_q_value(&self, agent_id: &str, key: &str, q_value: f64) -> Result<()> {
        use mongodb::bson::doc;

        let filter = doc! { "key": key };
        let update = doc! {
            "$set": {
                "key": key,
                "q_value": q_value,
                "last_updated_by": agent_id,
                "last_updated_at": chrono::Utc::now().timestamp(),
            },
            "$inc": {
                "update_count": 1
            }
        };

        let options = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();

        self.collection.update_one(filter, update, options).await?;

        Ok(())
    }

    /// Get learning statistics
    pub fn get_stats(&self) -> (usize, f64) {
        let q_count = self.q_table.len();
        let avg_q = if q_count > 0 {
            self.q_table.values().sum::<f64>() / q_count as f64
        } else {
            0.0
        };

        (q_count, avg_q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    struct TestState(i32);

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    enum TestAction {
        Up,
        Down,
    }

    #[test]
    fn test_hash_state_action() {
        let state = TestState(1);
        let action = TestAction::Up;
        let key = SharedQLearning::hash_state_action(&state, &action);

        assert!(!key.is_empty());
        assert!(key.contains("TestState"));
        assert!(key.contains("Up"));
    }
}
