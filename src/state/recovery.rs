use std::collections::HashMap;
use mongodb::{
    bson::{doc, to_bson},
    Client, Collection,
    options::{IndexOptions, FindOneOptions, FindOptions},
    IndexModel,
};
use futures_util::StreamExt;
use chrono::{DateTime, Utc, Duration};
use anyhow::{Result, anyhow};
use serde_json::Value;
use super::{PersistedState, StateTransition, StateRecovery};
use async_trait::async_trait;

pub struct RecoveryConfig {
    pub max_checkpoint_age: Duration,
    pub max_transitions_replay: i32,
    pub cleanup_older_than: Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_checkpoint_age: Duration::hours(24),
            max_transitions_replay: 100,
            cleanup_older_than: Duration::days(7),
        }
    }
}

pub struct StateRecoveryManager {
    checkpoints: Collection<PersistedState>,
    transitions: Collection<StateTransition>,
    states: Collection<PersistedState>,
    config: RecoveryConfig,
}

impl StateRecoveryManager {
    pub async fn new(client: &Client, config: RecoveryConfig) -> Result<Self> {
        let db = client.database("swarmonomicon");
        
        // Get collections
        let checkpoints = db.collection("state_checkpoints");
        let transitions = db.collection("state_transitions");
        let states = db.collection("agent_states");

        // Create indexes
        let checkpoint_index = IndexModel::builder()
            .keys(doc! { "agent_id": 1, "created_at": -1 })
            .options(Some(IndexOptions::builder().unique(false).build()))
            .build();
        checkpoints.create_index(checkpoint_index, None).await?;

        Ok(Self {
            checkpoints,
            transitions,
            states,
            config,
        })
    }

    async fn get_latest_checkpoint(&self, agent_id: &str) -> Result<Option<PersistedState>> {
        let filter = doc! {
            "agent_id": agent_id,
            "created_at": {
                "$gt": (Utc::now() - self.config.max_checkpoint_age).timestamp()
            }
        };
        let options = FindOneOptions::builder()
            .sort(doc! { "created_at": -1 })
            .build();
        Ok(self.checkpoints.find_one(filter, options).await?)
    }

    async fn get_transitions_since_checkpoint(
        &self,
        agent_id: &str,
        checkpoint_time: DateTime<Utc>,
    ) -> Result<Vec<StateTransition>> {
        let filter = doc! {
            "agent_id": agent_id,
            "timestamp": { "$gt": checkpoint_time.timestamp() }
        };
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": 1 })
            .limit(self.config.max_transitions_replay as i64)
            .build();

        let mut transitions = Vec::new();
        let mut cursor = self.transitions.find(filter, options).await?;
        while let Some(transition) = cursor.try_next().await? {
            transitions.push(transition);
        }
        Ok(transitions)
    }

    async fn apply_transition(
        &self,
        state: &mut PersistedState,
        transition: &StateTransition,
    ) -> Result<()> {
        // Validate transition
        if state.state_name != transition.from_state {
            return Err(anyhow!(
                "Invalid transition: state is in {} but transition is from {}",
                state.state_name,
                transition.from_state
            ));
        }

        // Apply transition
        state.state_name = transition.to_state.clone();
        state.updated_at = transition.timestamp;
        state.version += 1;

        // Add transition info to metadata
        state.metadata.insert(
            "last_transition".to_string(),
            serde_json::to_value(transition)?,
        );

        Ok(())
    }

    pub async fn cleanup_old_checkpoints(&self, agent_id: &str) -> Result<u64> {
        let filter = doc! {
            "agent_id": agent_id,
            "created_at": {
                "$lt": (Utc::now() - self.config.cleanup_older_than).timestamp()
            }
        };
        let result = self.checkpoints.delete_many(filter, None).await?;
        Ok(result.deleted_count)
    }
}

#[async_trait]
impl StateRecovery for StateRecoveryManager {
    async fn create_checkpoint(&self, state: &PersistedState) -> Result<()> {
        // Create a new checkpoint
        let mut checkpoint = state.clone();
        checkpoint.metadata.insert(
            "checkpoint_info".to_string(),
            serde_json::json!({
                "created_at": Utc::now().timestamp(),
                "original_version": state.version,
            }),
        );

        self.checkpoints.insert_one(checkpoint, None).await?;
        Ok(())
    }

    async fn rollback_to_checkpoint(&self, agent_id: &str) -> Result<Option<PersistedState>> {
        // Get latest valid checkpoint
        let checkpoint = self.get_latest_checkpoint(agent_id).await?;

        if let Some(checkpoint) = checkpoint {
            // Save current state as a new checkpoint before rolling back
            let current_state = self.states
                .find_one(doc! { "agent_id": agent_id }, None)
                .await?;

            if let Some(current) = current_state {
                let mut rollback_checkpoint = current.clone();
                rollback_checkpoint.metadata.insert(
                    "rollback_info".to_string(),
                    serde_json::json!({
                        "timestamp": Utc::now().timestamp(),
                        "reason": "Manual rollback to checkpoint",
                        "from_version": current.version,
                        "to_version": checkpoint.version,
                    }),
                );
                self.checkpoints.insert_one(rollback_checkpoint, None).await?;
            }

            // Update current state to checkpoint
            self.states
                .replace_one(doc! { "agent_id": agent_id }, checkpoint.clone(), None)
                .await?;

            Ok(Some(checkpoint))
        } else {
            Ok(None)
        }
    }

    async fn replay_transitions(&self, agent_id: &str, from_version: i32) -> Result<PersistedState> {
        // Get the state at the specified version
        let base_state = self.states
            .find_one(
                doc! {
                    "agent_id": agent_id,
                    "version": from_version
                },
                None,
            )
            .await?
            .ok_or_else(|| anyhow!("State not found for version {}", from_version))?;

        // Get all transitions since that version
        let transitions = self.get_transitions_since_checkpoint(agent_id, base_state.created_at).await?;

        // Apply transitions in sequence
        let mut current_state = base_state;
        for transition in transitions {
            if transition.success {
                self.apply_transition(&mut current_state, &transition).await?;
            }
        }

        // Save the replayed state
        current_state.metadata.insert(
            "replay_info".to_string(),
            serde_json::json!({
                "replayed_at": Utc::now().timestamp(),
                "from_version": from_version,
                "transitions_applied": transitions.len(),
            }),
        );

        self.states
            .replace_one(doc! { "agent_id": current_state.agent_id }, current_state.clone(), None)
            .await?;

        Ok(current_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_state_recovery() -> Result<()> {
        // Connect to test database
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let db = client.database("swarmonomicon_test");
        
        // Clear test collections
        db.collection::<PersistedState>("state_checkpoints").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;
        db.collection::<PersistedState>("agent_states").drop(None).await?;

        let config = RecoveryConfig {
            max_checkpoint_age: Duration::hours(1),
            max_transitions_replay: 10,
            cleanup_older_than: Duration::hours(24),
        };

        let manager = StateRecoveryManager::new(&client, config).await?;

        // Create test state
        let test_state = PersistedState {
            agent_id: "test_agent".to_string(),
            state_name: "initial".to_string(),
            state_data: None,
            conversation_context: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            metadata: HashMap::new(),
        };

        // Test checkpoint creation
        manager.create_checkpoint(&test_state).await?;
        let checkpoint = manager.get_latest_checkpoint("test_agent").await?;
        assert!(checkpoint.is_some());
        assert_eq!(checkpoint.unwrap().state_name, "initial");

        // Test transition replay
        let transition = StateTransition {
            id: "test_transition".to_string(),
            agent_id: "test_agent".to_string(),
            from_state: "initial".to_string(),
            to_state: "processing".to_string(),
            trigger: "test".to_string(),
            timestamp: Utc::now(),
            success: true,
            error: None,
        };

        manager.transitions.insert_one(transition, None).await?;
        manager.states.insert_one(test_state.clone(), None).await?;

        let replayed_state = manager.replay_transitions("test_agent", 1).await?;
        assert_eq!(replayed_state.state_name, "processing");
        assert_eq!(replayed_state.version, 2);

        // Test rollback
        let rolled_back = manager.rollback_to_checkpoint("test_agent").await?;
        assert!(rolled_back.is_some());
        let state = rolled_back.unwrap();
        assert_eq!(state.state_name, "initial");
        assert_eq!(state.version, 1);

        // Test cleanup
        let deleted = manager.cleanup_old_checkpoints("test_agent").await?;
        assert_eq!(deleted, 0); // No old checkpoints yet

        // Clean up
        db.collection::<PersistedState>("state_checkpoints").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;
        db.collection::<PersistedState>("agent_states").drop(None).await?;

        Ok(())
    }
} 
