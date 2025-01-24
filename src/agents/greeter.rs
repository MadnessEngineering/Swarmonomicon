use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine};

pub struct GreeterAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("greeting".to_string(), State {
                    prompt: "Would you like me to write a haiku for you?".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("yes".to_string(), "transfer_to_haiku".to_string());
                        transitions.insert("no".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("transfer_to_haiku".to_string(), State {
                    prompt: "Let me transfer you to our haiku expert...".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("goodbye".to_string(), State {
                    prompt: "Goodbye! Have a great day!".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states
            },
            initial_state: "greeting".to_string(),
        });

        let mut config = config;
        config.state_machine = state_machine.clone();
        
        Self { 
            config,
            state_manager: AgentStateManager::new(state_machine),
        }
    }

    fn create_response(&self, content: String) -> Message {
        Message {
            content,
            role: "assistant".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(MessageMetadata {
                tool_calls: None,
                state: self.state_manager.get_current_state().map(|s| s.prompt.clone()),
                confidence: Some(1.0),
            }),
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&mut self, message: &str) -> crate::Result<Message> {
        match self.state_manager.get_current_state_name() {
            Some("greeting") => {
                match message.to_lowercase().as_str() {
                    "yes" => {
                        self.state_manager.transition("yes");
                        Ok(self.create_response("Let me transfer you to our haiku expert...".to_string()))
                    }
                    "no" => {
                        self.state_manager.transition("no");
                        Ok(self.create_response("Goodbye! Have a great day!".to_string()))
                    }
                    _ => Ok(self.create_response("Would you like me to write a haiku for you? (yes/no)".to_string())),
                }
            }
            Some("transfer_to_haiku") => {
                Ok(self.create_response("Transferring to haiku agent...".to_string()))
            }
            Some("goodbye") => {
                Ok(self.create_response("Goodbye! Have a great day!".to_string()))
            }
            _ => {
                Ok(self.create_response("Would you like me to write a haiku for you? (yes/no)".to_string()))
            }
        }
    }

    async fn transfer_to(&mut self, agent_name: &str) -> crate::Result<()> {
        if !self.config.downstream_agents.contains(&agent_name.to_string()) {
            return Err("Invalid agent transfer target".into());
        }
        self.state_manager.transition("yes");
        Ok(())
    }

    async fn call_tool(&mut self, _tool: &crate::types::Tool, _params: HashMap<String, String>) -> crate::Result<String> {
        unimplemented!("Tool calling not yet implemented")
    }

    fn get_current_state(&self) -> Option<&State> {
        self.state_manager.get_current_state()
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Greets users".to_string(),
            instructions: "Greet the user".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeter_creation() {
        let config = create_test_config();
        let agent = GreeterAgent::new(config);
        assert!(agent.get_current_state().is_some());
    }

    #[tokio::test]
    async fn test_greeter_response() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        let response = agent.process_message("hi").await.unwrap();
        assert!(response.content.contains("haiku"));
        assert_eq!(response.role, "assistant");
        assert!(response.metadata.is_some());
    }

    #[tokio::test]
    async fn test_greeter_transfer() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        
        // Test valid transfer
        assert!(agent.transfer_to("haiku").await.is_ok());
        assert_eq!(
            agent.state_manager.get_current_state_name(),
            Some("transfer_to_haiku")
        );

        // Test invalid transfer
        assert!(agent.transfer_to("nonexistent").await.is_err());
    }
} 
