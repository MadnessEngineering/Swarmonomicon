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
    use crate::agents::GreeterAgent;
    use crate::types::AgentConfig;

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

        registry.register(agent).await.unwrap();
        let response = registry.get("test_greeter").unwrap()
            .process_message(Message::new("hi".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
    }

    #[cfg(feature = "haiku-agent")]
    #[tokio::test]
    async fn test_haiku_transfer() {
        let mut registry = AgentRegistry::new();
        let haiku = HaikuAgent::new(AgentConfig {
            name: "test_haiku".to_string(),
            public_description: "Test haiku agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register(haiku).await.unwrap();
        let response = registry.get("test_haiku").unwrap()
            .process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("haiku"));
    }
}
