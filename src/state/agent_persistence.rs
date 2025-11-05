use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use crate::types::{Message, AgentStateManager};
use super::{PersistedState, StateTransition, StatePersistence};

/// Helper for agents to persist their state
pub struct AgentStatePersistenceHelper<P: StatePersistence> {
    persistence: P,
}

impl<P: StatePersistence> AgentStatePersistenceHelper<P> {
    pub fn new(persistence: P) -> Self {
        Self { persistence }
    }

    /// Save the current agent state
    pub async fn save_agent_state(
        &self,
        state_manager: &AgentStateManager,
        conversation_context: Vec<Message>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let state = PersistedState {
            agent_id: state_manager.get_agent_id().to_string(),
            state_name: state_manager
                .get_current_state_name()
                .unwrap_or("default")
                .to_string(),
            state_data: None,
            conversation_context,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: state_manager.get_version(),
            metadata,
        };

        self.persistence.save_state(state).await
    }

    /// Load the agent state and restore it
    pub async fn load_agent_state(
        &self,
        agent_id: &str,
    ) -> Result<Option<(String, Vec<Message>, HashMap<String, serde_json::Value>)>> {
        if let Some(state) = self.persistence.load_state(agent_id).await? {
            Ok(Some((
                state.state_name,
                state.conversation_context,
                state.metadata,
            )))
        } else {
            Ok(None)
        }
    }

    /// Record a state transition
    pub async fn record_transition(
        &self,
        agent_id: &str,
        from_state: &str,
        to_state: &str,
        trigger: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<()> {
        let transition = StateTransition {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            from_state: from_state.to_string(),
            to_state: to_state.to_string(),
            trigger: trigger.to_string(),
            timestamp: Utc::now(),
            success,
            error,
        };

        self.persistence.record_transition(transition).await
    }

    /// Get all transitions for an agent
    pub async fn get_transitions(&self, agent_id: &str) -> Result<Vec<StateTransition>> {
        self.persistence.get_transitions(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::persistence::MongoPersistence;
    use mongodb::Client;

    #[tokio::test]
    async fn test_agent_persistence_helper() -> Result<()> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let persistence = MongoPersistence::new(&client).await?;
        let helper = AgentStatePersistenceHelper::new(persistence);

        // Create a test state manager
        let state_manager = AgentStateManager::new(None).with_agent_id("test_agent".to_string());

        // Save state
        helper
            .save_agent_state(&state_manager, vec![], HashMap::new())
            .await?;

        // Load state
        let loaded = helper.load_agent_state("test_agent").await?;
        assert!(loaded.is_some());

        // Record transition
        helper
            .record_transition("test_agent", "initial", "processing", "test", true, None)
            .await?;

        // Get transitions
        let transitions = helper.get_transitions("test_agent").await?;
        assert!(!transitions.is_empty());

        Ok(())
    }
}
