use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use mongodb::{
    bson::{doc, to_bson},
    Client, Collection,
    options::IndexOptions,
    IndexModel,
};
use crate::types::Message;
use anyhow::{Result, anyhow};
use serde_json::Value;

pub mod persistence;
pub mod validation;
pub mod recovery;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub agent_id: String,
    pub state_name: String,
    pub state_data: Option<Value>,
    pub conversation_context: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i32,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub id: String,
    pub agent_id: String,
    pub from_state: String,
    pub to_state: String,
    pub trigger: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error: Option<String>,
}

#[async_trait]
pub trait StatePersistence {
    async fn save_state(&self, state: PersistedState) -> Result<()>;
    async fn load_state(&self, agent_id: &str) -> Result<Option<PersistedState>>;
    async fn record_transition(&self, transition: StateTransition) -> Result<()>;
    async fn get_transitions(&self, agent_id: &str) -> Result<Vec<StateTransition>>;
}

pub trait StateValidator {
    fn validate_state(&self, state: &PersistedState) -> Result<()>;
    fn validate_transition(&self, from: &str, to: &str) -> Result<()>;
    fn validate_data(&self, state_data: &Value) -> Result<()>;
}

#[async_trait]
pub trait StateRecovery {
    async fn create_checkpoint(&self, state: &PersistedState) -> Result<()>;
    async fn rollback_to_checkpoint(&self, agent_id: &str) -> Result<Option<PersistedState>>;
    async fn replay_transitions(&self, agent_id: &str, from_version: i32) -> Result<PersistedState>;
}

pub struct MongoStateManager {
    states: Collection<PersistedState>,
    transitions: Collection<StateTransition>,
    checkpoints: Collection<PersistedState>,
}

impl MongoStateManager {
    pub async fn new(client: &Client) -> Result<Self> {
        let db = client.database("swarmonomicon");

        // Get collections
        let states = db.collection("agent_states");
        let transitions = db.collection("state_transitions");
        let checkpoints = db.collection("state_checkpoints");

        // Create indexes
        let state_index = IndexModel::builder()
            .keys(doc! { "agent_id": 1, "version": -1 })
            .options(Some(IndexOptions::builder().unique(true).build()))
            .build();
        states.create_index(state_index, None).await?;

        let transition_index = IndexModel::builder()
            .keys(doc! { "agent_id": 1, "timestamp": -1 })
            .build();
        transitions.create_index(transition_index, None).await?;

        let checkpoint_index = IndexModel::builder()
            .keys(doc! { "agent_id": 1, "version": -1 })
            .build();
        checkpoints.create_index(checkpoint_index, None).await?;

        Ok(Self {
            states,
            transitions,
            checkpoints,
        })
    }
}

#[async_trait]
impl StatePersistence for MongoStateManager {
    async fn save_state(&self, state: PersistedState) -> Result<()> {
        self.states.insert_one(state, None).await?;
        Ok(())
    }

    async fn load_state(&self, agent_id: &str) -> Result<Option<PersistedState>> {
        let filter = doc! { "agent_id": agent_id };
        let options = mongodb::options::FindOneOptions::builder()
            .sort(doc! { "version": -1 })
            .build();
        Ok(self.states.find_one(filter, options).await?)
    }

    async fn record_transition(&self, transition: StateTransition) -> Result<()> {
        self.transitions.insert_one(transition, None).await?;
        Ok(())
    }

    async fn get_transitions(&self, agent_id: &str) -> Result<Vec<StateTransition>> {
        let filter = doc! { "agent_id": agent_id };
        let options = mongodb::options::FindOptions::builder()
            .sort(doc! { "timestamp": 1 })
            .build();

        let mut transitions = Vec::new();
        let mut cursor = self.transitions.find(filter, options).await?;
        while let Some(transition) = cursor.try_next().await? {
            transitions.push(transition);
        }
        Ok(transitions)
    }
}

impl StateValidator for MongoStateManager {
    fn validate_state(&self, state: &PersistedState) -> Result<()> {
        // TODO: Implement state validation
        Ok(())
    }

    fn validate_transition(&self, from: &str, to: &str) -> Result<()> {
        // TODO: Implement transition validation
        Ok(())
    }

    fn validate_data(&self, state_data: &Value) -> Result<()> {
        // TODO: Implement data validation
        Ok(())
    }
}

#[async_trait]
impl StateRecovery for MongoStateManager {
    async fn create_checkpoint(&self, state: &PersistedState) -> Result<()> {
        self.checkpoints.insert_one(state.clone(), None).await?;
        Ok(())
    }

    async fn rollback_to_checkpoint(&self, agent_id: &str) -> Result<Option<PersistedState>> {
        let filter = doc! { "agent_id": agent_id };
        let options = mongodb::options::FindOneOptions::builder()
            .sort(doc! { "version": -1 })
            .build();
        Ok(self.checkpoints.find_one(filter, options).await?)
    }

    async fn replay_transitions(&self, agent_id: &str, from_version: i32) -> Result<PersistedState> {
        // TODO: Implement transition replay
        Err(anyhow!("Not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::state::persistence::MongoPersistence;
    use crate::state::validation::{StateValidationConfig, StateValidatorImpl};
    use crate::state::recovery::{RecoveryConfig, StateRecoveryManager};

    struct TestStateManager {
        persistence: MongoPersistence,
        validator: StateValidatorImpl,
        recovery: StateRecoveryManager,
    }

    #[async_trait]
    impl StatePersistence for TestStateManager {
        async fn save_state(&self, state: PersistedState) -> Result<()> {
            self.persistence.save_state(state).await
        }

        async fn load_state(&self, agent_id: &str) -> Result<Option<PersistedState>> {
            self.persistence.load_state(agent_id).await
        }

        async fn record_transition(&self, transition: StateTransition) -> Result<()> {
            self.persistence.record_transition(transition).await
        }

        async fn get_transitions(&self, agent_id: &str) -> Result<Vec<StateTransition>> {
            self.persistence.get_transitions(agent_id).await
        }
    }

    impl StateValidator for TestStateManager {
        fn validate_state(&self, state: &PersistedState) -> Result<()> {
            self.validator.validate_state(state)
        }

        fn validate_transition(&self, from: &str, to: &str) -> Result<()> {
            self.validator.validate_transition(from, to)
        }

        fn validate_data(&self, state_data: &Value) -> Result<()> {
            self.validator.validate_data(state_data)
        }
    }

    #[async_trait]
    impl StateRecovery for TestStateManager {
        async fn create_checkpoint(&self, state: &PersistedState) -> Result<()> {
            self.recovery.create_checkpoint(state).await
        }

        async fn rollback_to_checkpoint(&self, agent_id: &str) -> Result<Option<PersistedState>> {
            self.recovery.rollback_to_checkpoint(agent_id).await
        }

        async fn replay_transitions(&self, agent_id: &str, from_version: i32) -> Result<PersistedState> {
            self.recovery.replay_transitions(agent_id, from_version).await
        }
    }

    async fn create_test_manager() -> Result<TestStateManager> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let db = client.database("swarmonomicon_test");

        // Clear test collections
        db.collection::<PersistedState>("agent_states").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;
        db.collection::<PersistedState>("state_checkpoints").drop(None).await?;

        // Create validation config
        let mut validation_config = StateValidationConfig::new();
        validation_config.add_state("initial");
        validation_config.add_state("processing");
        validation_config.add_state("completed");
        validation_config.add_transition("initial", "processing");
        validation_config.add_transition("processing", "completed");

        Ok(TestStateManager {
            persistence: MongoPersistence::new(&client).await?,
            validator: StateValidatorImpl::new(validation_config),
            recovery: StateRecoveryManager::new(&client, RecoveryConfig::default()).await?,
        })
    }

    #[tokio::test]
    async fn test_state_management_integration() -> Result<()> {
        let manager = create_test_manager().await?;

        // Test 1: Create and validate initial state
        let initial_state = PersistedState {
            agent_id: "test_agent".to_string(),
            state_name: "initial".to_string(),
            state_data: None,
            conversation_context: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0,
            metadata: HashMap::new(),
        };

        // Validate state
        manager.validate_state(&initial_state)?;

        // Save state
        manager.save_state(initial_state.clone()).await?;

        // Create checkpoint
        manager.create_checkpoint(&initial_state).await?;

        // Test 2: Transition to processing state
        manager.validate_transition("initial", "processing")?;

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

        // Record transition
        manager.record_transition(transition).await?;

        // Test 3: Load and verify state
        let loaded_state = manager.load_state("test_agent").await?;
        assert!(loaded_state.is_some());
        let state = loaded_state.unwrap();
        assert_eq!(state.version, 1);
        assert_eq!(state.state_name, "initial");

        // Test 4: Replay transitions
        let replayed_state = manager.replay_transitions("test_agent", 1).await?;
        assert_eq!(replayed_state.state_name, "processing");
        assert_eq!(replayed_state.version, 2);

        // Test 5: Rollback to checkpoint
        let rolled_back = manager.rollback_to_checkpoint("test_agent").await?;
        assert!(rolled_back.is_some());
        let state = rolled_back.unwrap();
        assert_eq!(state.state_name, "initial");
        assert_eq!(state.version, 1);

        // Test 6: Verify transitions
        let transitions = manager.get_transitions("test_agent").await?;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from_state, "initial");
        assert_eq!(transitions[0].to_state, "processing");

        // Test 7: Invalid transition
        assert!(manager.validate_transition("initial", "completed").is_err());

        // Test 8: Invalid state
        let invalid_state = PersistedState {
            state_name: "invalid".to_string(),
            ..initial_state.clone()
        };
        assert!(manager.validate_state(&invalid_state).is_err());

        Ok(())
    }
}
