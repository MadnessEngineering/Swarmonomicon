mod greeter;
mod haiku;

pub use greeter::GreeterAgent;
pub use haiku::HaikuAgent;

use std::collections::HashMap;
use crate::types::{Agent, AgentConfig};
use crate::Result;

pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register<T: Agent + 'static>(&mut self, name: String, agent: T) {
        self.agents.insert(name, Box::new(agent));
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn Agent>> {
        self.agents.get_mut(name)
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Agent>> {
        self.agents.get(name)
    }

    pub fn create_default_agents(configs: Vec<AgentConfig>) -> Result<Self> {
        let mut registry = Self::new();
        
        for config in configs {
            match config.name.as_str() {
                "greeter" => registry.register(
                    config.name.clone(),
                    GreeterAgent::new(config),
                ),
                "haiku" => registry.register(
                    config.name.clone(),
                    HaikuAgent::new(config),
                ),
                _ => return Err(format!("Unknown agent type: {}", config.name).into()),
            }
        }

        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
} 
