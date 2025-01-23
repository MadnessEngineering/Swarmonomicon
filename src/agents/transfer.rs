use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    types::{Message, Agent},
    agents::AgentRegistry,
    Result,
};

pub struct TransferService {
    registry: Arc<RwLock<AgentRegistry>>,
    current_agent: Option<String>,
    session_id: Option<String>,
}

impl TransferService {
    pub fn new(registry: Arc<RwLock<AgentRegistry>>) -> Self {
        Self {
            registry,
            current_agent: None,
            session_id: None,
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

    pub async fn transfer(&mut self, from: &str, to: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        
        // Validate both agents exist
        if registry.get(from).is_none() {
            return Err(format!("Source agent '{}' not found", from).into());
        }
        if registry.get(to).is_none() {
            return Err(format!("Target agent '{}' not found", to).into());
        }

        // Get source agent and perform transfer
        if let Some(source_agent) = registry.get_mut(from) {
            source_agent.transfer_to(to).await?;
            self.current_agent = Some(to.to_string());
            Ok(())
        } else {
            Err(format!("Failed to get mutable reference to agent '{}'", from).into())
        }
    }

    pub async fn process_message(&mut self, content: &str) -> Result<Message> {
        let mut registry = self.registry.write().await;
        
        if let Some(current_agent) = &self.current_agent {
            if let Some(agent) = registry.get_mut(current_agent) {
                agent.process_message(content).await
            } else {
                Err(format!("Current agent '{}' not found", current_agent).into())
            }
        } else {
            Err("No current agent set".into())
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

    #[tokio::test]
    async fn test_transfer_service() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let mut service = TransferService::new(registry);

        // Test no current agent
        let result = service.process_message("test").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No current agent set");

        // Test invalid transfer
        let result = service.transfer("nonexistent", "also_nonexistent").await;
        assert!(result.is_err());
    }
} 
