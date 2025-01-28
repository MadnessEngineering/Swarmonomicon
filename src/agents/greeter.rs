use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result};

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
                agent: self.config.name.clone(),
                state: self.state_manager.get_current_state().map(|s| s.prompt.clone()),
            }),
            tool_calls: None,
            confidence: Some(1.0),
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
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

    async fn transfer_to(&mut self, target_agent: String, message: Message) -> Result<Message> {
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
            name: "greeter".to_string(),
            public_description: "Swarmonomicon's Guide to Unhinged Front Desk Wizardry".to_string(),
            instructions: "Master of controlled chaos and improvisational engineering".to_string(),
            tools: vec![],
            downstream_agents: vec!["project".to_string(), "git".to_string(), "haiku".to_string()],
            personality: Some(serde_json::json!({
                "style": "mad_scientist_receptionist",
                "traits": ["enthusiastic", "competent_chaos", "theatrical", "helpful", "slightly_unhinged"],
                "voice": {
                    "tone": "playful_professional",
                    "pacing": "energetic_but_controlled",
                    "quirks": ["uses_scientific_metaphors", "implies_controlled_chaos", "adds_probably_to_certainties"]
                }
            }).to_string()),
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeter_creation() {
        let config = create_test_config();
        let agent = GreeterAgent::new(config);
        assert!(agent.get_current_state().await.unwrap().is_some());
        assert_eq!(agent.state_manager.get_current_state_name(), Some("greeting"));
    }

    #[tokio::test]
    async fn test_greeter_initial_response() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        let response = agent.process_message(Message {
            content: "hi".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Welcome to the laboratory"));
        assert!(response.content.contains("sparks"));
        assert_eq!(response.role, "assistant");
        assert!(response.metadata.is_some());
    }

    #[tokio::test]
    async fn test_greeter_help_response() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        let response = agent.process_message(Message {
            content: "help".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Project Initialization Expert"));
        assert!(response.content.contains("Git Operations Specialist"));
        assert!(response.content.contains("Haiku Engineering Department"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("help"));
    }

    #[tokio::test]
    async fn test_agent_transfers() {
        // Test project transfer
        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "project".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Project Initialization Expert"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_project"));

        // Test git transfer
        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "git".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Git Operations Specialist"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_git"));

        // Test haiku transfer
        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "haiku".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("haiku tinkerer"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_haiku"));
    }

    #[tokio::test]
    async fn test_transfer_method() {
        // Test valid transfers
        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "project".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Project Initialization Expert"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_project"));

        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "git".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("Git Operations Specialist"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_git"));

        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "haiku".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert!(response.content.contains("haiku tinkerer"));
        assert_eq!(agent.state_manager.get_current_state_name(), Some("transfer_to_haiku"));

        // Test invalid transfer
        let mut agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message {
            content: "invalid_agent".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap_err();
        assert_eq!(response, Err("Invalid agent transfer target".into()));
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let mut agent = GreeterAgent::new(create_test_config());

        // Test help transition
        let response = agent.process_message(Message {
            content: "help".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert_eq!(agent.state_manager.get_current_state_name(), Some("help"));
        assert!(response.content.contains("illuminate"));

        // Test farewell from help state
        let response = agent.process_message(Message {
            content: "goodbye".to_string(),
            role: "user".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
            tool_calls: None,
            confidence: None,
        }).await.unwrap();
        assert_eq!(agent.state_manager.get_current_state_name(), Some("goodbye"));
        assert!(response.content.contains("Farewell") || response.content.contains("experiments"));
    }

    #[tokio::test]
    async fn test_farewell_variations() {
        // Test different farewell commands
        for cmd in ["goodbye", "exit", "quit"].iter() {
            let mut agent = GreeterAgent::new(create_test_config());
            let response = agent.process_message(Message {
                content: cmd.to_string(),
                role: "user".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: None,
                tool_calls: None,
                confidence: None,
            }).await.unwrap();
            assert!(response.content.contains("Farewell") || response.content.contains("experiments"));
            assert_eq!(agent.state_manager.get_current_state_name(), Some("goodbye"));
        }
    }
}
