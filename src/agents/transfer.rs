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

    pub async fn process_message(&mut self, message: Message) -> Result<Message> {
        if let Some(agent_name) = &self.current_agent {
            let registry = self.registry.read().await;
            if let Some(agent) = registry.get(agent_name) {
                let response = agent.process_message(message).await?;
                
                // Check if we need to transfer to another agent
                if let Some(metadata) = &response.metadata {
                    if let Some(target) = &metadata.transfer_target {
                        // Verify target agent exists before transferring
                        if registry.exists(target) {
                            self.current_agent = Some(target.clone());
                            return Ok(Message::new(format!("Transferring to {} agent...", target)));
                        } else {
                            return Err(format!("Target agent '{}' not found", target).into());
                        }
                    }
                }
                
                return Ok(response);
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
    use crate::types::AgentConfig;
    use crate::agents::greeter::GreeterAgent;

    #[tokio::test]
    async fn test_agent_transfer() {
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test_greeter".to_string(),
            public_description: "Test greeter agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec!["test_target".to_string()],
            personality: None,
            state_machine: None,
        });

        registry.register("test_greeter".to_string(), Box::new(agent)).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry);

        // Set current agent
        service.set_current_agent("test_greeter".to_string());

        // Process message that should trigger transfer
        let response = service.process_message(Message::new("transfer to test_target".to_string())).await;
        assert!(response.is_err()); // Should fail because test_target doesn't exist

        // Test manual transfer
        let result = service.transfer("test_greeter", "nonexistent").await;
        assert!(result.is_err());
    }
}
