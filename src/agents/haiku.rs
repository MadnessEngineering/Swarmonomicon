use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result};

pub struct HaikuAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
}

impl HaikuAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("awaiting_topic".to_string(), State {
                    prompt: "What would you like a haiku about?".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("topic_received".to_string(), "complete".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("complete".to_string(), State {
                    prompt: "Would you like another haiku?".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("yes".to_string(), "awaiting_topic".to_string());
                        transitions.insert("no".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: Some(ValidationRule {
                        pattern: "^(yes|no)$".to_string(),
                        error_message: "Please answer with 'yes' or 'no'".to_string(),
                    }),
                });
                states.insert("goodbye".to_string(), State {
                    prompt: "Thank you for listening to my haikus!".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states
            },
            initial_state: "awaiting_topic".to_string(),
        });

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
                agent: self.config.name.clone(),
                state: self.state_manager.get_current_state().map(|s| s.prompt.clone()),
            }),
            tool_calls: None,
            confidence: Some(1.0),
        }
    }

    fn generate_haiku(&self, topic: &str) -> String {
        // Mock haiku generation
        format!(
            "Topic: {}\n\nMocking haiku now\nNo API calls needed\nTests pass with ease",
            topic
        )
    }
}

#[async_trait]
impl Agent for HaikuAgent {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        // Mock haiku generation for now
        Ok(self.create_response(format!("Generated haiku:\n{}", message.content)))
    }

    async fn transfer_to(&mut self, target_agent: String, message: Message) -> Result<Message> {
        if !self.config.downstream_agents.contains(&target_agent) {
            return Err("Invalid agent transfer target".into());
        }
        Ok(message)
    }

    async fn call_tool(&mut self, _tool: &crate::types::Tool, _params: HashMap<String, String>) -> Result<String> {
        Err("Tool calling not yet implemented".into())
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(self.state_manager.get_current_state().cloned())
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus about any topic".to_string(),
            instructions: "Create haikus based on user topics".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_haiku_agent() {
        let mut agent = HaikuAgent::new(create_test_config());

        // Test message processing
        let response = agent.process_message(Message {
            content: "test".to_string(),
            role: "assistant".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(MessageMetadata {
                agent: "haiku".to_string(),
                state: Some("What would you like a haiku about?".to_string()),
            }),
            tool_calls: None,
            confidence: Some(1.0),
        }).await.unwrap();
        assert!(response.content.contains("Generated haiku:"));

        // Test config access
        let config = agent.get_config().await.unwrap();
        assert_eq!(config.name, "haiku");

        // Test state access
        let state = agent.get_current_state().await.unwrap();
        assert!(state.is_some());
    }
}
