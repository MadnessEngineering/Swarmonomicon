use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    types::{Message, Result, Agent},
    agents::AgentRegistry,
};
use anyhow::anyhow;
use std::collections::HashMap;

pub struct TransferService {
    registry: Arc<RwLock<AgentRegistry>>,
    current_agent: Option<String>,
}

impl TransferService {
    pub fn new(registry: Arc<RwLock<AgentRegistry>>) -> Self {
        Self {
            registry,
            current_agent: None,
        }
    }

    pub fn get_registry(&self) -> &Arc<RwLock<AgentRegistry>> {
        &self.registry
    }

    pub fn get_current_agent(&self) -> Option<&str> {
        self.current_agent.as_deref()
    }

    pub fn set_current_agent(&mut self, agent_name: String) {
        self.current_agent = Some(agent_name);
    }

    pub async fn process_message(&mut self, message: Message) -> Result<Message> {
        if let Some(agent_name) = &self.current_agent {
            let registry = self.registry.read().await;
            if let Some(agent) = registry.get_agent(agent_name) {
                let mut agent_lock = agent.write().await;
                agent_lock.process_message(message).await
            } else {
                Err(format!("Agent {} not found", agent_name).into())
            }
        } else {
            Err("No current agent set".into())
        }
    }

    pub async fn transfer(&mut self, source_agent: &str, target_agent: &str) -> Result<()> {
        let registry = self.registry.read().await;
        if let Some(source) = registry.get_agent(source_agent) {
            if let Some(target) = registry.get_agent(target_agent) {
                let source_lock = source.read().await;
                let message = Message::new(format!("Transferring from {} to {}", source_agent, target_agent));
                source_lock.transfer_to(target_agent.to_string(), message).await?;
                self.current_agent = Some(target_agent.to_string());
                Ok(())
            } else {
                Err(format!("Target agent {} not found", target_agent).into())
            }
        } else {
            Err(format!("Source agent {} not found", source_agent).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfig;
    use crate::agents::greeter::GreeterAgent;

    #[tokio::test]
    async fn test_transfer_service() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let mut service = TransferService::new(registry.clone());

        // Register test agents
        {
            let mut registry = registry.write().await;
            let greeter = GreeterAgent::new(AgentConfig {
                name: "test_greeter".to_string(),
                public_description: "Test greeter".to_string(),
                instructions: "Test greetings".to_string(),
                tools: vec![],
                downstream_agents: vec!["test_haiku".to_string()],
                personality: None,
                state_machine: None,
            });
            registry.register("test_greeter".to_string(), Box::new(greeter));

            let haiku = GreeterAgent::new(AgentConfig {
                name: "test_haiku".to_string(),
                public_description: "Test haiku".to_string(),
                instructions: "Test haiku generation".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });
            registry.register("test_haiku".to_string(), Box::new(haiku));
        }

        // Set initial agent
        service.set_current_agent("test_greeter".to_string());
        assert_eq!(service.get_current_agent(), Some("test_greeter"));

        // Test message processing
        let response = service.process_message(Message::new("hello".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));

        // Test transfer
        service.transfer("test_greeter", "test_haiku").await.unwrap();
        assert_eq!(service.get_current_agent(), Some("test_haiku"));

        // Test processing after transfer
        let response = service.process_message(Message::new("generate haiku".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
    }
}
