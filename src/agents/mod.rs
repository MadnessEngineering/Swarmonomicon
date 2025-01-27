use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::{Agent, AgentConfig, Result};

pub mod git_assistant;
pub mod greeter;
pub mod haiku;
pub mod project_init;
pub mod user_agent;

pub use git_assistant::GitAssistantAgent;
pub use greeter::GreeterAgent;
pub use haiku::HaikuAgent;
pub use project_init::ProjectInitAgent;
pub use user_agent::UserAgent;

#[derive(Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn Agent + Send + Sync>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register<A>(&mut self, agent: A) -> Result<()> 
    where
        A: Agent + Send + Sync + 'static,
    {
        let name = agent.get_config().name.clone();
        self.agents.insert(name, Box::new(agent));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&(dyn Agent + Send + Sync)> {
        self.agents.get(name).map(|agent| agent.as_ref())
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut (dyn Agent + Send + Sync)> {
        self.agents.get_mut(name).map(|agent| agent.as_mut())
    }

    pub fn create_default_agents(configs: Vec<AgentConfig>) -> Result<Self> {
        let mut registry = Self::new();
        
        for config in configs {
            match config.name.as_str() {
                "git" => registry.register(GitAssistantAgent::new(config))?,
                "project" => registry.register(ProjectInitAgent::new(config))?,
                "haiku" => registry.register(HaikuAgent::new(config))?,
                "greeter" => registry.register(GreeterAgent::new(config))?,
                "user" => registry.register(UserAgent::new(config))?,
                _ => return Err(format!("Unknown agent type: {}", config.name).into()),
            }
        }
        
        Ok(registry)
    }

    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}

// Global registry instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_REGISTRY: Arc<RwLock<AgentRegistry>> = Arc::new(RwLock::new(AgentRegistry::new()));
}

// Helper function to get the global registry
pub async fn get_registry() -> Arc<RwLock<AgentRegistry>> {
    GLOBAL_REGISTRY.clone()
}

// Helper function to register an agent globally
pub async fn register_agent<A>(agent: A) -> Result<()> 
where
    A: Agent + Send + Sync + 'static,
{
    let mut registry = GLOBAL_REGISTRY.write().await;
    registry.register(agent)
}

// Helper function to get an agent from the global registry
pub async fn get_agent(name: &str) -> Option<Box<dyn Agent + Send + Sync>> {
    let registry = GLOBAL_REGISTRY.read().await;
    registry.agents.get(name).map(|agent| agent.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_configs() -> Vec<AgentConfig> {
        vec![
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
        ]
    }

    #[tokio::test]
    async fn test_agent_registry() {
        let configs = create_test_configs();
        let mut registry = AgentRegistry::create_default_agents(configs).unwrap();
        
        // Test immutable access
        assert!(registry.get("greeter").is_some());
        assert!(registry.get("haiku").is_some());
        assert!(registry.get("nonexistent").is_none());

        // Test mutable access
        let greeter = registry.get_mut("greeter").unwrap();
        let response = greeter.process_message("hi").await.unwrap();
        assert!(response.content.contains("haiku"));

        // Test get all agents
        let all_agents = registry.list_agents();
        assert_eq!(all_agents.len(), 2);
    }

    #[tokio::test]
    async fn test_agent_workflow() {
        let configs = create_test_configs();
        let registry = AgentRegistry::create_default_agents(configs).unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry);

        // Start with greeter
        let response = service.process_message("hi").await;
        assert!(response.is_err()); // No current agent set

        // Set current agent to greeter and process message
        service.transfer("greeter", "haiku").await.unwrap();
        let response = service.process_message("nature").await.unwrap();
        assert!(response.content.contains("Mocking haiku now"));
    }
} 
