#![allow(warnings)]
pub mod agents;
pub mod tools;
pub mod config;
pub mod api;
pub mod error;
pub mod types;
pub mod ai;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Re-export commonly used types
pub use types::{Agent, AgentConfig, Message, Tool, State};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::RwLock;
    use std::sync::Arc;
    use crate::agents::{AgentRegistry, GitAgent, HaikuAgent};
    use crate::types::AgentConfig;

    #[tokio::test]
    async fn test_agent_registration() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        
        // Test git agent registration
        let git_config = AgentConfig {
            name: "git".to_string(),
            public_description: "Git operations".to_string(),
            instructions: "Handles git operations".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };
        let git_agent = GitAgent::new(git_config).await.unwrap();
        registry.write().await.register("git".to_string(), Box::new(git_agent));
        assert!(registry.read().await.get_agent("git").is_some());

        // Test haiku agent registration
        let haiku_config = AgentConfig {
            name: "haiku".to_string(),
            public_description: "Haiku creation".to_string(),
            instructions: "Creates haikus".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };
        registry.write().await.register("haiku".to_string(), Box::new(HaikuAgent::new(haiku_config)));
        assert!(registry.read().await.get_agent("haiku").is_some());
    }
}
