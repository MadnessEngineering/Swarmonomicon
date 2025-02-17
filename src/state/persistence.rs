use std::collections::HashMap;
use mongodb::{
    bson::{doc, to_bson},
    Client, Collection,
    options::{IndexOptions, FindOneOptions, FindOptions},
    IndexModel,
};
use futures_util::StreamExt;
use chrono::{DateTime, Utc};
use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::types::Message;
use super::{PersistedState, StateTransition, StatePersistence};
use async_trait::async_trait;

pub struct MongoPersistence {
    states: Collection<PersistedState>,
    transitions: Collection<StateTransition>,
}

impl MongoPersistence {
    pub async fn new(client: &Client) -> Result<Self> {
        let db = client.database("swarmonomicon");
        
        // Get collections
        let states = db.collection("agent_states");
        let transitions = db.collection("state_transitions");

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

        Ok(Self {
            states,
            transitions,
        })
    }

    pub async fn get_latest_version(&self, agent_id: &str) -> Result<i32> {
        let filter = doc! { "agent_id": agent_id };
        let options = FindOneOptions::builder()
            .sort(doc! { "version": -1 })
            .build();

        match self.states.find_one(filter, options).await? {
            Some(state) => Ok(state.version),
            None => Ok(0),
        }
    }

    pub async fn get_state_by_version(&self, agent_id: &str, version: i32) -> Result<Option<PersistedState>> {
        let filter = doc! {
            "agent_id": agent_id,
            "version": version
        };
        Ok(self.states.find_one(filter, None).await?)
    }

    pub async fn get_transitions_since(&self, agent_id: &str, since: DateTime<Utc>) -> Result<Vec<StateTransition>> {
        let filter = doc! {
            "agent_id": agent_id,
            "timestamp": { "$gt": since }
        };
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": 1 })
            .build();

        let mut transitions = Vec::new();
        let mut cursor = self.transitions.find(filter, options).await?;
        while let Some(transition) = cursor.try_next().await? {
            transitions.push(transition);
        }
        Ok(transitions)
    }

    pub async fn delete_old_states(&self, agent_id: &str, before_version: i32) -> Result<u64> {
        let filter = doc! {
            "agent_id": agent_id,
            "version": { "$lt": before_version }
        };
        let result = self.states.delete_many(filter, None).await?;
        Ok(result.deleted_count)
    }

    pub async fn delete_old_transitions(&self, agent_id: &str, before: DateTime<Utc>) -> Result<u64> {
        let filter = doc! {
            "agent_id": agent_id,
            "timestamp": { "$lt": before }
        };
        let result = self.transitions.delete_many(filter, None).await?;
        Ok(result.deleted_count)
    }
}

#[async_trait]
impl StatePersistence for MongoPersistence {
    async fn save_state(&self, state: PersistedState) -> Result<()> {
        // Get the latest version
        let latest_version = self.get_latest_version(&state.agent_id).await?;
        
        // Create new state with incremented version
        let mut new_state = state;
        new_state.version = latest_version + 1;
        new_state.updated_at = Utc::now();

        // Insert the new state
        self.states.insert_one(new_state, None).await?;
        Ok(())
    }

    async fn load_state(&self, agent_id: &str) -> Result<Option<PersistedState>> {
        let filter = doc! { "agent_id": agent_id };
        let options = FindOneOptions::builder()
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
        let options = FindOptions::builder()
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_mongo_persistence() -> Result<()> {
        // Connect to test database
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let db = client.database("swarmonomicon_test");
        
        // Clear test collections
        db.collection::<PersistedState>("agent_states").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;

        let persistence = MongoPersistence::new(&client).await?;

        // Test saving and loading state
        let test_state = PersistedState {
            agent_id: "test_agent".to_string(),
            state_name: "test_state".to_string(),
            state_data: None,
            conversation_context: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0, // Version will be set by save_state
            metadata: HashMap::new(),
        };

        persistence.save_state(test_state.clone()).await?;
        let loaded_state = persistence.load_state("test_agent").await?;
        assert!(loaded_state.is_some());
        let loaded = loaded_state.unwrap();
        assert_eq!(loaded.state_name, "test_state");
        assert_eq!(loaded.version, 1); // First version should be 1

        // Test version incrementing
        let mut second_state = test_state.clone();
        second_state.state_name = "test_state_2".to_string();
        persistence.save_state(second_state).await?;
        let latest = persistence.load_state("test_agent").await?.unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.state_name, "test_state_2");

        // Test transitions
        let now = Utc::now();
        let test_transition = StateTransition {
            id: "test_transition".to_string(),
            agent_id: "test_agent".to_string(),
            from_state: "state1".to_string(),
            to_state: "state2".to_string(),
            trigger: "test".to_string(),
            timestamp: now,
            success: true,
            error: None,
        };

        persistence.record_transition(test_transition.clone()).await?;
        
        // Test getting transitions since a time
        let transitions = persistence.get_transitions_since(
            "test_agent",
            now - Duration::seconds(1)
        ).await?;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from_state, "state1");

        // Test cleanup
        let deleted_states = persistence.delete_old_states("test_agent", 2).await?;
        assert_eq!(deleted_states, 1); // Should delete version 1

        let deleted_transitions = persistence.delete_old_transitions(
            "test_agent",
            now + Duration::seconds(1)
        ).await?;
        assert_eq!(deleted_transitions, 1); // Should delete our test transition

        // Clean up
        db.collection::<PersistedState>("agent_states").drop(None).await?;
        db.collection::<StateTransition>("state_transitions").drop(None).await?;

        Ok(())
    }
} 
