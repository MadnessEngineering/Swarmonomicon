use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Tool};
use crate::ai::{AiProvider, DefaultAiClient};
use anyhow::{Result, anyhow};
use std::error::Error as StdError;
use serde_json;

pub struct HaikuAgent {
    config: AgentConfig,
    state_manager: Arc<RwLock<AgentStateManager>>,
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
                        transitions.insert("topic_received".to_string(), "generating".to_string());
                        transitions
                    }),
                    validation: None,
                });
                states.insert("generating".to_string(), State {
                    name: "generating".to_string(),
                    data: None,
                    prompt: Some("🎋 Weaving your thoughts into digital poetry...".to_string()),
                    transitions: Some({
                        let mut transitions = HashMap::new();
                        transitions.insert("haiku_generated".to_string(), "complete".to_string());
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
            state_manager: Arc::new(RwLock::new(AgentStateManager::new(state_machine))),
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

        let mut haiku = self.ai_client.chat(system_prompt, messages.clone()).await?;

        // Validate haiku format
        let lines: Vec<&str> = haiku.trim().split('\n').collect();
        if lines.len() != 3 {
            return Err(anyhow!("Generated haiku does not have 3 lines"));
        }

        let syllables: Vec<usize> = lines.iter().map(|line| count_syllables(line)).collect();
        if syllables != vec![5, 7, 5] {
            // Regenerate haiku with retry limit
            let mut retry_count = 0;
            let retry_limit = 5;
            let mut best_attempt = haiku.clone();
            let mut best_syllables = syllables.clone();

            while retry_count < retry_limit {
                haiku = self.ai_client.chat(system_prompt, messages.clone()).await?;
                let lines: Vec<&str> = haiku.trim().split('\n').collect();
                let syllables: Vec<usize> = lines.iter().map(|line| count_syllables(line)).collect();
                if syllables == vec![5, 7, 5] {
                    break;
                }
                // Check if this attempt is closer to 5-7-5 than the current best
                if syllables.iter().zip(&[5, 7, 5]).map(|(a,b)| a-b).sum::<usize>()
                    < best_syllables.iter().zip(&[5, 7, 5]).map(|(a,b)| a-b).sum::<usize>() {
                    best_attempt = haiku.clone();
                    best_syllables = syllables.clone();
                }
                retry_count += 1;
            }

            if retry_count == retry_limit {
                log::warn!("Hit retry limit generating 5-7-5 haiku. Returning best attempt.");
                haiku = best_attempt;
            }
        }

        Ok(haiku)
    }

    async fn create_response(&self, content: String) -> Message {
        let guard = self.state_manager.read().await;
        let current_state = guard.get_current_state_name();
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

fn count_syllables(line: &str) -> usize {
    // Simple syllable counting logic
    line.to_lowercase()
        .split(|c: char| !(c.is_alphabetic() || c == '\''))
        .filter(|s| !s.is_empty())
        .map(|word| word.chars().filter(|c| "aeiouy".contains(*c)).count())
        .sum()
}

#[async_trait]
impl Agent for HaikuAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let guard = self.state_manager.read().await;
        let state = guard.get_current_state_name().map(|s| s.to_string());
        drop(guard); // Drop the read guard before acquiring write guards

        let response = match state {
            Some(state) => match state.as_str() {
                "awaiting_topic" => {
                    // Store the topic in the state data
                    let mut state_manager = self.state_manager.write().await;
                    let mut current_state = state_manager.get_current_state()
                        .ok_or_else(|| anyhow!("Failed to get current state"))?
                        .clone();

                    // Create state data with the topic
                    current_state.data = Some(message.content.clone());

                    // Transition to generating state
                    state_manager.transition("topic_received")
                        .ok_or_else(|| anyhow!("Failed to transition to generating state"))?;

                    self.create_response("🎋 Weaving your thoughts into digital poetry...".to_string()).await
                }
                "generating" => {
                    // Get the stored topic
                    let state_manager = self.state_manager.read().await;
                    let current_state = state_manager.get_current_state()
                        .ok_or_else(|| anyhow!("Failed to get current state"))?;
                    let topic = match &current_state.data {
                        Some(data) => data.clone(),
                        None => message.content.clone()
                    };
                    drop(state_manager);

                    // Generate the haiku
                    let haiku = self.generate_haiku(topic).await?;

                    // Transition to complete state
                    self.state_manager.write().await.transition("haiku_generated")
                        .ok_or_else(|| anyhow!("Failed to transition to complete state"))?;

                    self.create_response(haiku).await
                }
                "complete" => {
                    match message.content.to_lowercase().as_str() {
                        "yes" => {
                            self.state_manager.write().await.transition("yes")
                                .ok_or_else(|| anyhow!("Failed to transition to awaiting_topic state"))?;
                            self.create_response("🌸 What new topic shall inspire our next algorithmic verse?".to_string()).await
                        }
                        "no" => {
                            self.state_manager.write().await.transition("no")
                                .ok_or_else(|| anyhow!("Failed to transition to goodbye state"))?;
                            self.create_response("🌟 May your path be illuminated by the glow of poetic algorithms...".to_string()).await
                        }
                        _ => self.create_response("Please respond with 'yes' to continue or 'no' to conclude.".to_string()).await,
                    }
                }
                "goodbye" => self.create_response("Farewell, seeker of digital poetry.".to_string()).await,
                _ => return Err(anyhow!("Invalid state: {}", state)),
            },
            None => self.create_response("🌸 What shall we crystallize into algorithmic verse today?".to_string()).await,
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
        Ok(self.state_manager.read().await.get_current_state().cloned())
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockAiClient;

    #[async_trait]
    impl AiProvider for MockAiClient {
        async fn chat(&self, _system_prompt: &str, _messages: Vec<HashMap<String, String>>) -> Result<String> {
            Ok("Digital petals fall\nSilicon dreams take their flight\nCode blooms in the night".to_string())
        }
    }

    fn create_test_state_machine() -> StateMachine {
        StateMachine {
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
                states.insert("goodbye".to_string(), State {
                    name: "goodbye".to_string(),
                    data: None,
                    prompt: Some("Farewell!".to_string()),
                    transitions: None,
                    validation: None,
                });
                states
            },
            initial_state: "awaiting_topic".to_string(),
        }
    }

    #[tokio::test]
    async fn test_haiku_generation() {
        let mut agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Test haiku agent".to_string(),
            instructions: "Test haiku generation".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: Some(create_test_state_machine()),
        });

        // Replace the default AI client with our mock
        agent = agent.with_ai_client(MockAiClient);

        // First message transitions to generating state and stores the topic
        let response = agent.process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("Weaving"), "Should transition to generating state");

        // Second message generates the haiku
        let response = agent.process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("Digital petals"), "Response should contain a haiku");

        // Verify we're in the complete state
        let state = agent.get_current_state().await.unwrap();
        assert_eq!(state.unwrap().name, "complete");
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
