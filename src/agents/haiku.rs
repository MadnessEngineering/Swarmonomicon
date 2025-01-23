use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule};

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
    async fn process_message(&mut self, message: &str) -> crate::Result<Message> {
        match self.state_manager.get_current_state_name() {
            Some("awaiting_topic") => {
                let haiku = self.generate_haiku(message);
                self.state_manager.transition("topic_received");
                Ok(self.create_response(haiku))
            }
            Some("complete") => {
                match message.to_lowercase().as_str() {
                    "yes" => {
                        self.state_manager.transition("yes");
                        Ok(self.create_response("What would you like a haiku about?".to_string()))
                    }
                    "no" => {
                        self.state_manager.transition("no");
                        Ok(self.create_response("Thank you for listening to my haikus!".to_string()))
                    }
                    _ => Ok(self.create_response("Please answer with 'yes' or 'no'.".to_string())),
                }
            }
            Some("goodbye") => {
                Ok(self.create_response("Thank you for listening to my haikus!".to_string()))
            }
            _ => {
                self.state_manager.transition("topic_received");
                Ok(self.create_response("What would you like a haiku about?".to_string()))
            }
        }
    }

    async fn transfer_to(&mut self, agent_name: &str) -> crate::Result<()> {
        if !self.config.downstream_agents.contains(&agent_name.to_string()) {
            return Err("Invalid agent transfer target".into());
        }
        unimplemented!("Agent transfer mechanism not yet implemented")
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
    async fn test_haiku_creation() {
        let config = create_test_config();
        let agent = HaikuAgent::new(config);
        assert!(agent.get_current_state().is_some());
        assert_eq!(
            agent.get_current_state().unwrap().prompt,
            "What would you like a haiku about?"
        );
    }

    #[tokio::test]
    async fn test_haiku_generation() {
        let config = create_test_config();
        let mut agent = HaikuAgent::new(config);
        let response = agent.process_message("nature").await.unwrap();
        assert!(response.content.contains("Mocking haiku now"));
        assert!(response.content.contains("Tests pass with ease"));
        assert_eq!(response.role, "assistant");
    }

    // #[tokio::test]
    // async fn test_haiku_flow() {
    //     let config = create_test_config();
    //     let mut agent = HaikuAgent::new(config);

    //     // Initial state should be awaiting_topic
    //     assert_eq!(agent.state_manager.get_current_state_name(), Some("awaiting_topic"));

    //     // First haiku
    //     let response = agent.process_message("moon").await.unwrap();
    //     assert!(response.content.contains("Mocking haiku now"));
    //     assert_eq!(agent.state_manager.get_current_state_name(), Some("complete"));

    //     // Ask for another
    //     let response = agent.process_message("yes").await.unwrap();
    //     assert_eq!(response.content, "What would you like a haiku about?");
    //     assert_eq!(agent.state_manager.get_current_state_name(), Some("awaiting_topic"));

    //     // Say goodbye
    //     let response = agent.process_message("no").await.unwrap();
    //     assert!(response.content.contains("Thank you"));
    //     assert_eq!(agent.state_manager.get_current_state_name(), Some("goodbye"));
    // }
}
