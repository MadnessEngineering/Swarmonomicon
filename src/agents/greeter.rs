use async_trait::async_trait;
use crate::types::{Agent, AgentConfig};

pub struct GreeterAgent {
    config: AgentConfig,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&self, _message: &str) -> crate::Result<String> {
        unimplemented!("Greeter agent message processing not yet implemented")
    }

    async fn transfer_to(&self, _agent_name: &str) -> crate::Result<()> {
        unimplemented!("Agent transfer not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_greeter_creation() {
        let config = AgentConfig {
            name: "greeter".to_string(),
            public_description: "Greets users".to_string(),
            instructions: "Greet the user".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
        };
        
        let _agent = GreeterAgent::new(config);
        // More tests will be added as we implement functionality
    }
} 
