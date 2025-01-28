use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    types::{Message, Result, Agent},
    agents::AgentRegistry,
};

pub struct TransferService {
    current_agent: Option<String>,
    registry: Arc<RwLock<AgentRegistry>>,
}

impl TransferService {
    pub fn new(registry: Arc<RwLock<AgentRegistry>>) -> Self {
        Self {
            current_agent: None,
            registry,
        }
    }

    pub fn get_registry(&self) -> &Arc<RwLock<AgentRegistry>> {
        &self.registry
    }

    pub fn get_current_agent(&self) -> Option<&str> {
        self.current_agent.as_deref()
    }

    pub fn set_current_agent(&mut self, agent: String) {
        self.current_agent = Some(agent);
    }

    pub async fn process_message(&self, message: Message) -> Result<Message> {
        if let Some(agent_name) = &self.current_agent {
            let registry = self.registry.read().await;
            if let Some(agent) = registry.get(agent_name) {
                return agent.process_message(message).await;
            }
        }
        Err("No current agent set".into())
    }

    pub async fn transfer(&mut self, from: &str, to: &str) -> Result<()> {
        let registry = self.registry.read().await;
        if registry.get(to).is_some() {
            self.current_agent = Some(to.to_string());
            Ok(())
        } else {
            Err(format!("Agent {} not found", to).into())
        }
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    pub fn get_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfig;
    use crate::agents::{AgentRegistry, GreeterAgent, HaikuAgent};

    #[tokio::test]
    async fn test_transfer_service() {
        // Create a test registry
        let mut registry = AgentRegistry::new();

        // Add test agents
        let greeter = GreeterAgent::new(AgentConfig {
            name: "greeter".to_string(),
            public_description: "Test greeter".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        });

        let haiku = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Test haiku".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register(greeter).unwrap();
        registry.register(haiku).unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry);

        // Test no current agent
        let result = service.process_message("test").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No current agent set");

        // Test setting current agent and processing message
        service.set_current_agent("greeter".to_string());
        let result = service.process_message("hi").await;
        assert!(result.is_ok());

        // Test transfer
        let result = service.transfer("greeter", "haiku").await;
        assert!(result.is_ok());
        assert_eq!(service.get_current_agent(), Some("haiku"));

        // Test invalid transfer
        let result = service.transfer("nonexistent", "haiku").await;
        assert!(result.is_err());
    }
}
