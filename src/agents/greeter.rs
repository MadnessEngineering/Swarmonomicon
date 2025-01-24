use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, AgentResponse};
use crate::Result;

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
                    prompt: "Initial welcome to the workshop of wonders".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("help".to_string(), "help".to_string());
                        transitions.insert("transfer".to_string(), "transfer".to_string());
                        transitions.insert("farewell".to_string(), "farewell".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("help".to_string(), State {
                    prompt: "Responding to questions about capabilities".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("transfer".to_string(), "transfer".to_string());
                        transitions.insert("greeting".to_string(), "greeting".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("transfer".to_string(), State {
                    prompt: "Transferring to specialist agents".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("farewell".to_string(), State {
                    prompt: "Bidding farewell to visitors".to_string(),
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

    fn get_response_for_state(&self, state: &str) -> String {
        match state {
            "greeting" => {
                let greetings = [
                    "Ah, welcome to the Swarmonomicon's workshop of wonders! Let's see what we can tinker with today.",
                    "Greetings, fellow experimenter! You've reached the front desk of controlled chaos.",
                    "Welcome to the laboratory! Don't mind the sparks, they're mostly decorative.",
                    "Step right in! The mad science is perfectly calibrated today... probably.",
                ];
                greetings[fastrand::usize(..greetings.len())].to_string()
            },
            "help" => {
                let help_responses = [
                    "Ah, seeking guidance through our labyrinth of possibilities! Let me illuminate our various tools and specialists:\n\
                     - Project Initialization Expert: For creating new experiments and research spaces\n\
                     - Git Operations Specialist: For managing and documenting our mad science\n\
                     - Haiku Engineering Department: For when you need your chaos in 5-7-5 format",
                    "Questions! Excellent! That's how all the best mad science begins. We have several departments of expertise:\n\
                     - Project Creation Lab: Where new ideas take shape\n\
                     - Git Version Control Chamber: For experiment versioning\n\
                     - Haiku Synthesis Station: Poetry meets technology",
                    "Let me illuminate the path through our wonderful chaos! We've got tools and agents for all sorts of fascinating experiments:\n\
                     - Project Foundry: Where ideas become reality\n\
                     - Git Time Machine: Track and manage your mad science\n\
                     - Haiku Generator: Technical poetry in motion",
                ];
                help_responses[fastrand::usize(..help_responses.len())].to_string()
            },
            "transfer" => {
                let transfer_responses = [
                    "Ah, this looks like a job for our specialized tinkerer! Let me transfer you to the right department...",
                    "I know just the mad scientist for this experiment! Allow me to redirect you...",
                    "This requires our expert in that particular form of chaos! One moment while I make the connection...",
                ];
                transfer_responses[fastrand::usize(..transfer_responses.len())].to_string()
            },
            "farewell" => {
                let farewell_responses = [
                    "Off to new experiments! Remember: if something explodes, it was definitely intentional!",
                    "Until our next grand collaboration! Keep those gears turning!",
                    "Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!",
                ];
                farewell_responses[fastrand::usize(..farewell_responses.len())].to_string()
            },
            _ => "The machines are humming nicely - what shall we create?".to_string(),
        }
    }

    fn determine_state_transition(&self, input: &str) -> &str {
        let input_lower = input.to_lowercase();
        
        if input_lower.contains("bye") || input_lower.contains("goodbye") {
            "farewell"
        } else if input_lower.contains("help") || input_lower.contains("what") || input_lower.contains("how") {
            "help"
        } else if self.should_transfer(input) {
            "transfer"
        } else {
            "greeting"
        }
    }

    fn should_transfer(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        input_lower.contains("project") ||
        input_lower.contains("create") ||
        input_lower.contains("git") ||
        input_lower.contains("commit") ||
        input_lower.contains("haiku")
    }

    fn determine_transfer_target(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();
        if input_lower.contains("project") || input_lower.contains("create") {
            Some("project".to_string())
        } else if input_lower.contains("git") || input_lower.contains("commit") {
            Some("git".to_string())
        } else if input_lower.contains("haiku") {
            Some("haiku".to_string())
        } else {
            None
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        let next_state = self.determine_state_transition(message);
        let response = self.get_response_for_state(next_state);
        let should_transfer = self.should_transfer(message);
        let transfer_to = if should_transfer {
            self.determine_transfer_target(message)
        } else {
            None
        };

        Ok(AgentResponse {
            content: if should_transfer {
                format!("{}\n\n{}", response, self.get_response_for_state("transfer"))
            } else {
                response
            },
            should_transfer,
            transfer_to,
        })
    }

    async fn transfer_to(&mut self, agent_name: &str) -> Result<()> {
        if !self.config.downstream_agents.contains(&agent_name.to_string()) {
            return Err("Invalid agent transfer target".into());
        }
        self.state_manager.transition("transfer");
        Ok(())
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    fn get_current_state(&self) -> Option<&State> {
        self.state_manager.get_current_state()
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
            })),
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
        let agent = GreeterAgent::new(config);
        
        // Test greeting
        let response = agent.process_message("hi").await.unwrap();
        assert!(response.content.contains("workshop") || response.content.contains("laboratory"));
        
        // Test help
        let response = agent.process_message("help").await.unwrap();
        assert!(response.content.contains("specialists") || response.content.contains("departments"));
        
        // Test transfer
        let response = agent.process_message("I need to create a project").await.unwrap();
        assert!(response.content.contains("transfer"));
        assert!(response.should_transfer);
        assert_eq!(response.transfer_to, Some("project".to_string()));
    }

    #[tokio::test]
    async fn test_greeter_transfer() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);

        // Test valid transfers
        assert!(agent.transfer_to("project").await.is_ok());
        assert!(agent.transfer_to("git").await.is_ok());
        assert!(agent.transfer_to("haiku").await.is_ok());

        // Test invalid transfer
        assert!(agent.transfer_to("nonexistent").await.is_err());
    }
}
