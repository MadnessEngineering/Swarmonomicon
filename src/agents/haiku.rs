use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result, ToolCall, Tool};

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
        }
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
            ])
            .with_context(HashMap::new());

        Message {
            content,
            role: Some("assistant".to_string()),
            timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64),
            metadata: Some(metadata),
        }
    }

    fn generate_haiku(&self, topic: String) -> String {
        // In a real implementation, this would use more sophisticated haiku generation
        // For now, we'll return themed mock haikus based on the topic
        let haikus = vec![
            format!(
                "🌸 {} flows soft\nThrough quantum gates of spring code\nPatterns emerge now",
                topic
            ),
            format!(
                "🍁 Digital leaves\nFloat through {} streams of thought\nAlgorithms bloom",
                topic
            ),
            format!(
                "⚡ {} sparks bright\nIn binary gardens grow\nPoetic functions",
                topic
            ),
            format!(
                "🌿 Nature's patterns\nMeet {} in code space\nHarmony achieved",
                topic
            ),
        ];

        // Select a haiku based on a hash of the topic
        let index = topic.bytes().sum::<u8>() as usize % haikus.len();
        haikus[index].clone()
    }
}

#[async_trait]
impl Agent for HaikuAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        // Generate a haiku response
        let haiku = self.generate_haiku(message.content);
        Ok(Message::new(haiku))
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Ok(format!("Called tool {} with params {:?}", tool.name, params))
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(None)
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
    async fn test_state_transitions() {
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
            }),
        });

        let state = agent.get_current_state().await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().name, "awaiting_topic");

        let response = agent.process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("haiku"));

        let state = agent.get_current_state().await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().name, "complete");
    }
}
