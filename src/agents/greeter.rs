use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager};

pub struct GreeterAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_manager = AgentStateManager::new(config.state_machine.clone());
        Self { 
            config,
            state_manager,
        }
    }

    fn create_response(&self, content: String) -> Message {
        Message {
            content,
            role: "assistant".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(MessageMetadata {
                tool_calls: None,
                state: self.state_manager.get_current_state().map(|s| s.prompt.clone()),
                confidence: Some(1.0),
            }),
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&mut self, _message: &str) -> crate::Result<Message> {
        // For now, just return a simple greeting
        Ok(self.create_response(
            "Hello! Would you like me to write a haiku for you?".to_string()
        ))
    }

    async fn transfer_to(&mut self, agent_name: &str) -> crate::Result<()> {
        if !self.config.downstream_agents.contains(&agent_name.to_string()) {
            return Err("Invalid agent transfer target".into());
        }
        unimplemented!("Agent transfer mechanism not yet implemented")
    }

    async fn call_tool(&mut self, _tool: &crate::types::Tool, _params: HashMap<String, String>) -> crate::Result<String> {
        unimplemented!("Tool calling not yet implemented")
    }

    fn get_current_state(&self) -> Option<&State> {
        self.state_manager.get_current_state()
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Greets users".to_string(),
            instructions: "Greet the user".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeter_creation() {
        let config = create_test_config();
        let agent = GreeterAgent::new(config);
        assert!(agent.get_current_state().is_none());
    }

    #[tokio::test]
    async fn test_greeter_response() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        let response = agent.process_message("hi").await.unwrap();
        assert!(response.content.contains("haiku"));
        assert_eq!(response.role, "assistant");
        assert!(response.metadata.is_some());
    }
} 
