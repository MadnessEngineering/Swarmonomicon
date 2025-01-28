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
                states.insert("greeting".to_string(), State {
                    prompt: "Welcome to the laboratory! Don't mind the sparks, they're mostly decorative.".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("help".to_string(), "help".to_string());
                        transitions.insert("project".to_string(), "transfer_to_project".to_string());
                        transitions.insert("git".to_string(), "transfer_to_git".to_string());
                        transitions.insert("haiku".to_string(), "transfer_to_haiku".to_string());
                        transitions.insert("farewell".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("help".to_string(), State {
                    prompt: "Questions! Excellent! That's how all the best mad science begins.".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("project".to_string(), "transfer_to_project".to_string());
                        transitions.insert("git".to_string(), "transfer_to_git".to_string());
                        transitions.insert("haiku".to_string(), "transfer_to_haiku".to_string());
                        transitions.insert("farewell".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("transfer_to_project".to_string(), State {
                    prompt: "Ah, a new experiment needs initialization! Let me summon our Project Initialization Expert...".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("transfer_to_git".to_string(), State {
                    prompt: "Time for some version control wizardry! Connecting you to our Git Operations Specialist...".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("transfer_to_haiku".to_string(), State {
                    prompt: "Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("goodbye".to_string(), State {
                    prompt: "Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("awaiting_input".to_string(), State {
                    name: Some("awaiting_input".to_string()),
                    data: None,
                    prompt: Some("👋 Hello! I'm your friendly greeter. How can I assist you today?".to_string()),
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
                    name: Some("project_transfer".to_string()),
                    data: None,
                    prompt: Some("🚀 Let me connect you with the Project Assistant...".to_string()),
                    transitions: Some(HashMap::new()),
                    validation: None,
                });
                states.insert("git_transfer".to_string(), State {
                    name: Some("git_transfer".to_string()),
                    data: None,
                    prompt: Some("🌳 Transferring you to the Git Assistant...".to_string()),
                    transitions: Some(HashMap::new()),
                    validation: None,
                });
                states.insert("haiku_transfer".to_string(), State {
                    name: Some("haiku_transfer".to_string()),
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
            timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64),
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
        match self.state_manager.get_current_state_name() {
            Some("greeting") => {
                match message.content.to_lowercase().as_str() {
                    "help" => {
                        self.state_manager.transition("help");
                        Ok(self.create_response("Let me illuminate the path through our wonderful chaos! We've got tools and agents for all sorts of fascinating experiments:\n- Project Initialization Expert: For creating new experiments and research spaces\n- Git Operations Specialist: For managing and documenting our mad science\n- Haiku Engineering Department: For when you need your chaos in 5-7-5 format".to_string()))
                    }
                    "project" => {
                        self.state_manager.transition("project");
                        Ok(self.create_response("Ah, a new experiment needs initialization! Let me summon our Project Initialization Expert...".to_string()))
                    }
                    "git" => {
                        self.state_manager.transition("git");
                        Ok(self.create_response("Time for some version control wizardry! Connecting you to our Git Operations Specialist...".to_string()))
                    }
                    "haiku" => {
                        self.state_manager.transition("haiku");
                        Ok(self.create_response("Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string()))
                    }
                    "goodbye" | "exit" | "quit" => {
                        self.state_manager.transition("farewell");
                        Ok(self.create_response("Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string()))
                    }
                    _ => Ok(self.create_response("Welcome to the laboratory! Don't mind the sparks, they're mostly decorative. How may I assist with your experiments? (Try: 'help', 'project', 'git', 'haiku', or 'goodbye')".to_string())),
                }
            }
            Some("help") => {
                match message.content.to_lowercase().as_str() {
                    "project" => {
                        self.state_manager.transition("project");
                        Ok(self.create_response("Ah, a new experiment needs initialization! Let me summon our Project Initialization Expert...".to_string()))
                    }
                    "git" => {
                        self.state_manager.transition("git");
                        Ok(self.create_response("Time for some version control wizardry! Connecting you to our Git Operations Specialist...".to_string()))
                    }
                    "haiku" => {
                        self.state_manager.transition("haiku");
                        Ok(self.create_response("Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string()))
                    }
                    "goodbye" | "exit" | "quit" => {
                        self.state_manager.transition("farewell");
                        Ok(self.create_response("Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string()))
                    }
                    _ => Ok(self.create_response("Let me illuminate our specialist departments! We have experts in project creation, git operations, and haiku engineering! (Try: 'project', 'git', 'haiku', or 'goodbye')".to_string())),
                }
            }
            Some("transfer_to_project") => {
                Ok(self.create_response("Initializing project matrices... connecting you to our Project Initialization Expert!".to_string()))
            }
            Some("transfer_to_git") => {
                Ok(self.create_response("Branching into the version control dimension... connecting you to our Git Operations Specialist!".to_string()))
            }
            Some("transfer_to_haiku") => {
                Ok(self.create_response("Calibrating the haiku matrices... transferring you to our resident verse engineer!".to_string()))
            }
            Some("goodbye") => {
                Ok(self.create_response("Off to new experiments! Remember: if something explodes, it was definitely intentional!".to_string()))
            }
            _ => {
                Ok(self.create_response("Welcome to the laboratory! Don't mind the sparks, they're mostly decorative. How may I assist you today? (Try: 'help', 'project', 'git', 'haiku', or 'goodbye')".to_string()))
            }
        }
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        if !self.config.downstream_agents.contains(&target_agent) {
            return Err("Invalid agent transfer target".into());
        }
        match target_agent.as_str() {
            "project" => {
                self.state_manager.transition("project");
                Ok(message)
            },
            "git" => {
                self.state_manager.transition("git");
                Ok(message)
            },
            "haiku" => {
                self.state_manager.transition("haiku");
                Ok(message)
            },
            _ => Err("Invalid agent transfer target".into()),
        }
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
