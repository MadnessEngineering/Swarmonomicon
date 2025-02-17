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

    #[tokio::test]
    async fn test_state_persistence() -> Result<()> {
        // Connect to test database
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let db = client.database("swarmonomicon_test");
        
        // Clear test collections
        db.collection::<PersistedState>("agent_states").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;
        db.collection::<PersistedState>("state_checkpoints").drop(None).await?;

        let manager = MongoStateManager::new(&client).await?;

        // Test saving and loading state
        let test_state = PersistedState {
            agent_id: "test_agent".to_string(),
            state_name: "test_state".to_string(),
            state_data: None,
            conversation_context: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            metadata: HashMap::new(),
        };

        manager.save_state(test_state.clone()).await?;
        let loaded_state = manager.load_state("test_agent").await?;
        assert!(loaded_state.is_some());
        assert_eq!(loaded_state.unwrap().state_name, "test_state");

        // Test recording and retrieving transitions
        let test_transition = StateTransition {
            id: "test_transition".to_string(),
            agent_id: "test_agent".to_string(),
            from_state: "state1".to_string(),
            to_state: "state2".to_string(),
            trigger: "test".to_string(),
            timestamp: Utc::now(),
            success: true,
            error: None,
        };

        manager.record_transition(test_transition.clone()).await?;
        let transitions = manager.get_transitions("test_agent").await?;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from_state, "state1");

        // Clean up
        db.collection::<PersistedState>("agent_states").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;
        db.collection::<PersistedState>("state_checkpoints").drop(None).await?;

        Ok(())
    }
} 
