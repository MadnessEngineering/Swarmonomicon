use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result, ToolCall, Tool};

pub struct GreeterAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("awaiting_input".to_string(), State {
                    name: "awaiting_input".to_string(),
                    data: None,
                    prompt: Some("Welcome to the laboratory! Don't mind the sparks, they're mostly decorative.".to_string()),
                    transitions: Some({
                        let mut transitions = HashMap::new();
                        transitions.insert("project".to_string(), "project_transfer".to_string());
                        transitions.insert("git".to_string(), "git_transfer".to_string());
                        transitions.insert("haiku".to_string(), "haiku_transfer".to_string());
                        transitions
                    }),
                    validation: None,
                });
                states.insert("project_transfer".to_string(), State {
                    name: "project_transfer".to_string(),
                    data: None,
                    prompt: Some("🚀 Let me connect you with the Project Assistant...".to_string()),
                    transitions: Some(HashMap::new()),
                    validation: None,
                });
                states.insert("git_transfer".to_string(), State {
                    name: "git_transfer".to_string(),
                    data: None,
                    prompt: Some("🌳 Transferring you to the Git Assistant...".to_string()),
                    transitions: Some(HashMap::new()),
                    validation: None,
                });
                states.insert("haiku_transfer".to_string(), State {
                    name: "haiku_transfer".to_string(),
                    data: None,
                    prompt: Some("🌸 Connecting you with the Haiku Assistant...".to_string()),
                    transitions: Some(HashMap::new()),
                    validation: None,
                });
                states
            },
            initial_state: "awaiting_input".to_string(),
        });

        let mut config = config;
        config.state_machine = state_machine.clone();

        Self {
            config,
            state_manager: AgentStateManager::new(state_machine),
        }
    }

    fn create_response(&self, content: String) -> Message {
        let current_state = self.state_manager.get_current_state_name();
        let metadata = MessageMetadata::new(self.config.name.clone())
            .with_state(current_state.unwrap_or("awaiting_input").to_string())
            .with_personality(vec![
                "friendly".to_string(),
                "helpful".to_string(),
                "welcoming".to_string(),
            ]);

        Message {
            content,
            role: Some("assistant".to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
            metadata: Some(metadata),
        }
    }

    fn should_transfer(&self, message: &str) -> Option<String> {
        match message.to_lowercase().as_str() {
            "project" => Some("project".to_string()),
            "git" => Some("git".to_string()),
            "haiku" => Some("haiku".to_string()),
            _ => None,
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let current_state = self.state_manager.get_current_state_name();
        let response = match current_state {
            Some("awaiting_input") => {
                match message.content.as_str() {
                    "project" => {
                        let metadata = MessageMetadata::new(self.config.name.clone())
                            .with_state("project_transfer".to_string())
                            .with_transfer("project".to_string())
                            .with_personality(vec![
                                "friendly".to_string(),
                                "helpful".to_string(),
                                "welcoming".to_string(),
                            ]);
                        Message {
                            content: "🚀 Let me connect you with the Project Assistant...".to_string(),
                            role: Some("assistant".to_string()),
                            timestamp: Some(chrono::Utc::now().timestamp()),
                            metadata: Some(metadata),
                        }
                    }
                    "git" => {
                        let metadata = MessageMetadata::new(self.config.name.clone())
                            .with_state("git_transfer".to_string())
                            .with_transfer("git".to_string())
                            .with_personality(vec![
                                "friendly".to_string(),
                                "helpful".to_string(),
                                "welcoming".to_string(),
                            ]);
                        Message {
                            content: "🌳 Transferring you to the Git Assistant...".to_string(),
                            role: Some("assistant".to_string()),
                            timestamp: Some(chrono::Utc::now().timestamp()),
                            metadata: Some(metadata),
                        }
                    }
                    "haiku" => {
                        let metadata = MessageMetadata::new(self.config.name.clone())
                            .with_state("haiku_transfer".to_string())
                            .with_transfer("haiku".to_string())
                            .with_personality(vec![
                                "friendly".to_string(),
                                "helpful".to_string(),
                                "welcoming".to_string(),
                            ]);
                        Message {
                            content: "🌸 Connecting you with the Haiku Assistant...".to_string(),
                            role: Some("assistant".to_string()),
                            timestamp: Some(chrono::Utc::now().timestamp()),
                            metadata: Some(metadata),
                        }
                    }
                    _ => Message {
                        content: "👋 Hello! I'm your friendly greeter. How can I assist you today?".to_string(),
                        role: Some("assistant".to_string()),
                        timestamp: Some(chrono::Utc::now().timestamp()),
                        metadata: Some(MessageMetadata::new(self.config.name.clone())
                            .with_state(current_state.unwrap_or("awaiting_input").to_string())
                            .with_personality(vec![
                                "friendly".to_string(),
                                "helpful".to_string(),
                                "welcoming".to_string(),
                            ])),
                    },
                }
            }
            _ => Message {
                content: "👋 Hello! I'm your friendly greeter. How can I assist you today?".to_string(),
                role: Some("assistant".to_string()),
                timestamp: Some(chrono::Utc::now().timestamp()),
                metadata: Some(MessageMetadata::new(self.config.name.clone())
                    .with_state(current_state.unwrap_or("awaiting_input").to_string())
                    .with_personality(vec![
                        "friendly".to_string(),
                        "helpful".to_string(),
                        "welcoming".to_string(),
                    ])),
            },
        };
        Ok(response)
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        if !self.config.downstream_agents.contains(&target_agent) {
            return Err("Invalid agent transfer target".into());
        }
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Ok(format!("Called tool {} with params {:?}", tool.name, params))
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
            name: "greeter".to_string(),
            public_description: "Friendly greeter agent".to_string(),
            instructions: "Greet users and direct them to appropriate agents".to_string(),
            tools: vec![],
            downstream_agents: vec![
                "project".to_string(),
                "git".to_string(),
                "haiku".to_string(),
            ],
            personality: Some(serde_json::json!({
                "style": "friendly_receptionist",
                "traits": ["friendly", "helpful", "welcoming"],
                "voice": {
                    "tone": "warm_and_professional",
                    "pacing": "measured",
                    "quirks": ["uses_emojis", "enthusiastic_greetings"]
                }
            }).to_string()),
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeting() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("hi".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.agent, "greeter");
            assert!(metadata.personality_traits.is_some());
        }
    }

    #[tokio::test]
    async fn test_project_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("project".to_string())).await.unwrap();
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("project".to_string()));
        }
    }

    #[tokio::test]
    async fn test_git_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("git".to_string())).await.unwrap();
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("git".to_string()));
        }
    }

    #[tokio::test]
    async fn test_haiku_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("haiku".to_string())).await.unwrap();
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("haiku".to_string()));
        }
    }

    #[tokio::test]
    async fn test_invalid_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let result = agent.transfer_to("invalid".to_string(), Message::new("test".to_string())).await;
        assert!(result.is_err());
    }
}
