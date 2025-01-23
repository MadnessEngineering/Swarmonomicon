use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::{Message};
use crate::Result;
use super::AgentRegistry;

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

    pub async fn transfer(&mut self, from: &str, to: &str) -> Result<()> {
        let registry = self.registry.read().await;
        
        // Verify both agents exist
        if registry.get(from).is_none() || registry.get(to).is_none() {
            return Err("One or both agents not found".into());
        }

        // Verify the transfer is allowed
        let from_agent = registry.get(from).unwrap();
        let downstream_agents = &from_agent.get_config().downstream_agents;
        
        if !downstream_agents.contains(&to.to_string()) {
            return Err("Transfer not allowed: agent not in downstream agents".into());
        }

        // Update current agent
        self.current_agent = Some(to.to_string());
        Ok(())
    }

    pub async fn process_message(&mut self, message: &str) -> Result<Message> {
        let mut registry = self.registry.write().await;
        
        if let Some(current_agent) = &self.current_agent {
            if let Some(agent) = registry.get_mut(current_agent) {
                agent.process_message(message).await
            } else {
                Err("Current agent not found".into())
            }
        } else {
            Err("No current agent set".into())
        }
    }

    pub fn get_current_agent(&self) -> Option<&str> {
        self.current_agent.as_deref()
    }

    pub fn get_registry(&self) -> &Arc<RwLock<AgentRegistry>> {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfig;

    #[tokio::test]
    async fn test_transfer_service() {
        // Create test agents
        let configs = vec![
            AgentConfig {
                name: "greeter".to_string(),
                public_description: "Greets users".to_string(),
                instructions: "Greet the user".to_string(),
                tools: vec![],
                downstream_agents: vec!["haiku".to_string()],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "haiku".to_string(),
                public_description: "Creates haikus".to_string(),
                instructions: "Create haikus".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
        ];

        let registry = AgentRegistry::create_default_agents(configs).unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry);

        // Test invalid transfer
        assert!(service.transfer("nonexistent", "haiku").await.is_err());
        assert!(service.transfer("haiku", "nonexistent").await.is_err());
        assert!(service.transfer("haiku", "greeter").await.is_err());

        // Test valid transfer
        assert!(service.transfer("greeter", "haiku").await.is_ok());
        assert_eq!(service.get_current_agent(), Some("haiku"));
    }
} 
