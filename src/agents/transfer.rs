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
        
        if !registry.exists(from) {
            return Err(format!("Source agent '{}' not found", from).into());
        }
        
        if !registry.exists(to) {
            return Err(format!("Target agent '{}' not found", to).into());
        }

        self.current_agent = Some(to.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{GreeterAgent, HaikuAgent};
    use crate::types::AgentConfig;

    #[tokio::test]
    async fn test_transfer_service() {
        let mut registry = AgentRegistry::new();
        
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

        registry.register(greeter).await.unwrap();
        registry.register(haiku).await.unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry);

        // Test initial state
        assert!(service.get_current_agent().is_none());

        // Test setting current agent
        service.set_current_agent("greeter".to_string());
        assert_eq!(service.get_current_agent(), Some("greeter"));

        // Test message processing
        let result = service.process_message(Message::new("test".to_string())).await;
        assert!(result.is_ok());

        // Test transfer
        let result = service.transfer("greeter", "haiku").await;
        assert!(result.is_ok());
        assert_eq!(service.get_current_agent(), Some("haiku"));

        // Test message after transfer
        let result = service.process_message(Message::new("hi".to_string())).await;
        assert!(result.is_ok());
    }
}
