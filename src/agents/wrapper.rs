use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, Message, Tool, State, AgentConfig, Result};

/// A wrapper type that handles the complexity of agent type management.
/// This provides a consistent interface for working with agents while
/// handling the necessary thread-safety and dynamic dispatch requirements.
pub struct AgentWrapper {
    inner: Box<dyn Agent + Send + Sync>,
}

impl AgentWrapper {
    /// Create a new AgentWrapper from any type that implements Agent
    pub fn new(agent: Box<dyn Agent + Send + Sync>) -> Self {
        Self { inner: agent }
    }
}

#[async_trait]
impl Agent for AgentWrapper {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        self.inner.process_message(message).await
    }

    async fn transfer_to(&mut self, target_agent: String, message: Message) -> Result<Message> {
        self.inner.transfer_to(target_agent, message).await
    }

    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.inner.call_tool(tool, params).await
    }

    async fn get_current_state(&self) -> Result<Option<String>> {
        self.inner.get_current_state().await.map(|state| state.map(|s| s.name))
    }

    async fn get_config(&self) -> Result<String> {
        self.inner.get_config().await.map(|config| serde_json::to_string(&config).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::haiku::HaikuAgent;

    #[tokio::test]
    async fn test_agent_wrapper() {
        let config = AgentConfig {
            name: "test".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let agent = HaikuAgent::new(config.clone());
        let mut wrapper = AgentWrapper::new(Box::new(agent));

        // Test that we can get the config
        let config = wrapper.get_config().await.unwrap();
        assert_eq!(config.name, "test");

        // Test that we can process messages
        let response = wrapper.process_message("test").await;
        assert!(response.is_ok());

        // Test cloning
        let wrapper2 = wrapper.clone();
        let config2 = wrapper2.get_config().await.unwrap();
        assert_eq!(config2.name, "test");

        // Test state access
        let state = wrapper.get_current_state().await;
        assert!(state.is_ok());
    }
} 
