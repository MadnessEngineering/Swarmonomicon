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

    #[tokio::test]
    async fn test_agent_registration() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        
        // Test git agent registration
        registry.write().await.register("git".to_string(), Box::new(GitAgent::new()));
        assert!(registry.read().await.get("git").is_some());

        // Test haiku agent registration
        registry.write().await.register("haiku".to_string(), Box::new(HaikuAgent::new()));
        assert!(registry.read().await.get("haiku").is_some());
    }
}
