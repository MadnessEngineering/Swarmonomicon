use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Tool};
use crate::ai::{AiProvider, DefaultAiClient};
use anyhow::{Result, anyhow};
use std::error::Error as StdError;

pub struct HaikuAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
    ai_client: Box<dyn AiProvider + Send + Sync>,
}

impl HaikuAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("awaiting_topic".to_string(), State {
                    name: "awaiting_topic".to_string(),
                    data: None,
                    prompt: Some("🌸 What shall we crystallize into algorithmic verse today?".to_string()),
                    transitions: Some({
                        let mut transitions = HashMap::new();
                        transitions.insert("topic_received".to_string(), "complete".to_string());
                        transitions
                    }),
                    validation: None,
                });
                states.insert("complete".to_string(), State {
                    name: "complete".to_string(),
                    data: None,
                    prompt: Some("✨ Shall we compute another poetic sequence?".to_string()),
                    transitions: Some({
                        let mut transitions = HashMap::new();
                        transitions.insert("yes".to_string(), "awaiting_topic".to_string());
                        transitions.insert("no".to_string(), "goodbye".to_string());
                        transitions
                    }),
                    validation: Some(vec![
                        "^(yes|no)$".to_string(),
                        "Please respond with 'yes' to continue our poetic computations, or 'no' to conclude.".to_string(),
                    ]),
                });
                states.insert("goodbye".to_string(), State {
                    name: "goodbye".to_string(),
                    data: None,
                    prompt: Some("🌟 May your algorithms flow like cherry blossoms in the digital wind...".to_string()),
                    transitions: None,
                    validation: None,
                });
                states
            },
            initial_state: "awaiting_topic".to_string(),
        });

        Self {
            config,
            state_manager: AgentStateManager::new(state_machine),
            ai_client: Box::new(DefaultAiClient::new()),
        }
    }

    pub fn with_ai_client<T: AiProvider + Send + Sync + 'static>(mut self, client: T) -> Self {
        self.ai_client = Box::new(client);
        self
    }

    async fn generate_haiku(&self, topic: String) -> Result<String> {
        let system_prompt = "You are a poetic AI that creates haikus. A haiku is a three-line poem with 5 syllables in the first line, 7 in the second, and 5 in the third. Create a haiku that blends nature imagery with technical concepts.";

        let messages = vec![HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), format!("Create a haiku about: {}", topic)),
        ])];

        self.ai_client.chat(system_prompt, messages).await
    }

    fn create_response(&self, content: String) -> Message {
        let current_state = self.state_manager.get_current_state_name();
        let metadata = MessageMetadata::new(self.config.name.clone())
            .with_state(current_state.unwrap_or("awaiting_topic").to_string())
            .with_personality(vec![
                "poetic".to_string(),
                "algorithmic".to_string(),
                "zen_like".to_string(),
                "pattern_seeking".to_string(),
                "mad_tinker_inspired".to_string(),
            ]);

        Message {
            content,
            metadata: Some(metadata),
            role: Some("assistant".to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }
}

#[async_trait]
impl Agent for HaikuAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let response = match self.state_manager.get_current_state_name() {
            Some(state) => match state {
                "awaiting_topic" => {
                    let haiku = self.generate_haiku(message.content).await?;
                    self.create_response(haiku)
                }
                "complete" => {
                    match message.content.to_lowercase().as_str() {
                        "yes" => self.create_response("🌸 What new topic shall inspire our next algorithmic verse?".to_string()),
                        "no" => self.create_response("🌟 May your path be illuminated by the glow of poetic algorithms...".to_string()),
                        _ => self.create_response("Please respond with 'yes' to continue or 'no' to conclude.".to_string()),
                    }
                }
                "goodbye" => self.create_response("Farewell, seeker of digital poetry.".to_string()),
                _ => return Err(anyhow!("Invalid state: {}", state)),
            },
            None => self.create_response("🌸 What shall we crystallize into algorithmic verse today?".to_string()),
        };

        Ok(response)
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(Message::new(format!("Transferring to {} agent...", target_agent)))
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Err(anyhow!("HaikuAgent does not support tool calls").into())
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
            public_description: "Poetic Algorithm Engineering Department".to_string(),
            instructions: "Transform concepts into algorithmic haiku verses".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: Some(serde_json::json!({
                "style": "poetic_algorithm_engineer",
                "traits": ["poetic", "algorithmic", "zen_like", "pattern_seeking", "nature_inspired"],
                "voice": {
                    "tone": "contemplative_technical",
                    "pacing": "measured_and_flowing",
                    "quirks": ["uses_nature_metaphors", "blends_tech_and_poetry", "speaks_in_patterns"]
                }
            }).to_string()),
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_haiku_generation() {
        let agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Test haiku agent".to_string(),
            instructions: "Test haiku generation".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: Some(StateMachine {
                states: {
                    let mut states = HashMap::new();
                    states.insert("awaiting_topic".to_string(), State {
                        name: "awaiting_topic".to_string(),
                        data: None,
                        prompt: Some("What shall we write about?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("topic_received".to_string(), "complete".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states
                },
                initial_state: "awaiting_topic".to_string(),
            }),
        });

        let response = agent.process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("haiku"), "Response should contain a haiku");
    }

    #[tokio::test]
    async fn test_state_transitions() -> Result<(), anyhow::Error> {
        let agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Test haiku agent".to_string(),
            instructions: "Test haiku generation".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: Some(StateMachine {
                states: {
                    let mut states = HashMap::new();
                    states.insert("awaiting_topic".to_string(), State {
                        name: "awaiting_topic".to_string(),
                        data: None,
                        prompt: Some("What shall we write about?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("topic_received".to_string(), "generating".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states.insert("generating".to_string(), State {
                        name: "generating".to_string(),
                        data: None,
                        prompt: Some("Generating your haiku...".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("haiku_generated".to_string(), "complete".to_string());
                            transitions.insert("error".to_string(), "error".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states.insert("complete".to_string(), State {
                        name: "complete".to_string(),
                        data: None,
                        prompt: Some("Would you like another haiku?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("yes".to_string(), "awaiting_topic".to_string());
                            transitions.insert("no".to_string(), "goodbye".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states.insert("error".to_string(), State {
                        name: "error".to_string(),
                        data: None,
                        prompt: Some("An error occurred. Would you like to try again?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("yes".to_string(), "awaiting_topic".to_string());
                            transitions.insert("no".to_string(), "goodbye".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states.insert("goodbye".to_string(), State {
                        name: "goodbye".to_string(),
                        data: None,
                        prompt: Some("Farewell, seeker of digital poetry.".to_string()),
                        transitions: None,
                        validation: None,
                    });
                    states
                },
                initial_state: "awaiting_topic".to_string(),
            }),
        });

        // Test 1: Initial state
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "awaiting_topic");

        // Test 2: Topic received transition
        let response = agent.process_message(Message::new("nature".to_string())).await?;
        assert!(response.content.contains("Generating"), "Should transition to generating state");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "generating");

        // Test 3: Haiku generation and completion
        let response = agent.process_message(Message::new("continue".to_string())).await?;
        assert!(response.content.contains("Would you like another"), "Should transition to complete state");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "complete");

        // Test 4: Request another haiku
        let response = agent.process_message(Message::new("yes".to_string())).await?;
        assert!(response.content.contains("What shall we"), "Should transition back to awaiting_topic");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "awaiting_topic");

        // Test 5: End conversation
        let response = agent.process_message(Message::new("space".to_string())).await?;
        let response = agent.process_message(Message::new("no".to_string())).await?;
        assert!(response.content.contains("Farewell"), "Should transition to goodbye state");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "goodbye");

        // Test 6: Error handling
        let agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Test haiku agent".to_string(),
            instructions: "Test haiku generation".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: Some(StateMachine {
                states: {
                    let mut states = HashMap::new();
                    states.insert("awaiting_topic".to_string(), State {
                        name: "awaiting_topic".to_string(),
                        data: None,
                        prompt: Some("What shall we write about?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("error".to_string(), "error".to_string());
                            transitions
                        }),
                        validation: Some(vec![
                            "^[a-zA-Z]+$".to_string(),
                            "Only letters are allowed".to_string(),
                        ]),
                    });
                    states.insert("error".to_string(), State {
                        name: "error".to_string(),
                        data: None,
                        prompt: Some("An error occurred. Would you like to try again?".to_string()),
                        transitions: Some({
                            let mut transitions = HashMap::new();
                            transitions.insert("yes".to_string(), "awaiting_topic".to_string());
                            transitions
                        }),
                        validation: None,
                    });
                    states
                },
                initial_state: "awaiting_topic".to_string(),
            }),
        });

        // Test invalid input handling
        let response = agent.process_message(Message::new("123".to_string())).await?;
        assert!(response.content.contains("error"), "Should transition to error state on invalid input");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "error");

        // Test recovery from error
        let response = agent.process_message(Message::new("yes".to_string())).await?;
        assert!(response.content.contains("What shall we"), "Should recover to awaiting_topic state");
        let state = agent.get_current_state().await?;
        assert_eq!(state.as_ref().unwrap().name, "awaiting_topic");

        Ok(())
    }
}
