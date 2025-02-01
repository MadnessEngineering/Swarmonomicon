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
    use crate::types::{Message, State, StateMachine, AgentStateManager};

    #[tokio::test]
    async fn test_transfer_service() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let mut service = TransferService::new(registry.clone());
        
        service.set_current_agent("greeter".to_string());
        assert_eq!(service.get_current_agent(), Some("greeter"));

        service.transfer("greeter", "haiku").await.unwrap();
        assert_eq!(service.get_current_agent(), Some("haiku"));
    }
}
