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
pub mod agent_persistence;

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
        // Validate required fields
        if state.agent_id.is_empty() {
            return Err(anyhow!("Agent ID cannot be empty"));
        }

        if state.state_name.is_empty() {
            return Err(anyhow!("State name cannot be empty"));
        }

        // Validate version is non-negative
        if state.version < 0 {
            return Err(anyhow!("State version cannot be negative"));
        }

        // Validate timestamps
        if state.updated_at < state.created_at {
            return Err(anyhow!("Updated timestamp cannot be before created timestamp"));
        }

        // Validate state_data if present
        if let Some(data) = &state.state_data {
            self.validate_data(data)?;
        }

        Ok(())
    }

    fn validate_transition(&self, from: &str, to: &str) -> Result<()> {
        // Validate transition parameters
        if from.is_empty() {
            return Err(anyhow!("Source state cannot be empty"));
        }

        if to.is_empty() {
            return Err(anyhow!("Target state cannot be empty"));
        }

        // Prevent self-transitions (optional rule, can be removed if needed)
        if from == to {
            return Err(anyhow!("State cannot transition to itself: {}", from));
        }

        Ok(())
    }

    fn validate_data(&self, state_data: &Value) -> Result<()> {
        // Basic validation that data is properly formed
        match state_data {
            Value::Object(map) => {
                // Ensure all keys are valid strings
                for (key, value) in map {
                    if key.is_empty() {
                        return Err(anyhow!("State data keys cannot be empty"));
                    }

                    // Recursively validate nested objects
                    if let Value::Object(_) = value {
                        self.validate_data(value)?;
                    }
                }
                Ok(())
            },
            Value::Array(arr) => {
                // Validate array elements
                for item in arr {
                    if let Value::Object(_) = item {
                        self.validate_data(item)?;
                    }
                }
                Ok(())
            },
            _ => Ok(()) // Primitive values are always valid
        }
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
        // Get the base state at the specified version
        let filter = doc! {
            "agent_id": agent_id,
            "version": from_version
        };
        let mut current_state = self.states
            .find_one(filter, None)
            .await?
            .ok_or_else(|| anyhow!("State not found for agent {} at version {}", agent_id, from_version))?;

        // Get all transitions after this version's timestamp
        let transition_filter = doc! {
            "agent_id": agent_id,
            "timestamp": { "$gt": current_state.updated_at.timestamp() }
        };
        let options = mongodb::options::FindOptions::builder()
            .sort(doc! { "timestamp": 1 })
            .build();

        let mut cursor = self.transitions.find(transition_filter, options).await?;

        // Replay each successful transition
        let mut transitions_applied = 0;
        while let Some(transition) = cursor.try_next().await? {
            if transition.success {
                // Validate the transition
                self.validate_transition(&transition.from_state, &transition.to_state)?;

                // Ensure we're in the expected state before applying transition
                if current_state.state_name != transition.from_state {
                    return Err(anyhow!(
                        "Invalid transition replay: expected state '{}' but found '{}'",
                        transition.from_state,
                        current_state.state_name
                    ));
                }

                // Apply the transition
                current_state.state_name = transition.to_state.clone();
                current_state.updated_at = transition.timestamp;
                current_state.version += 1;

                // Add transition info to metadata
                current_state.metadata.insert(
                    "last_transition".to_string(),
                    serde_json::to_value(&transition)?,
                );

                transitions_applied += 1;
            }
        }

        // Add replay metadata
        current_state.metadata.insert(
            "replay_info".to_string(),
            serde_json::json!({
                "replayed_at": chrono::Utc::now().timestamp(),
                "from_version": from_version,
                "transitions_applied": transitions_applied,
            }),
        );

        // Validate the final state before saving
        self.validate_state(&current_state)?;

        // Save the replayed state
        self.save_state(current_state.clone()).await?;

        Ok(current_state)
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
