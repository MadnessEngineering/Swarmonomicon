use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;
use tokio::sync::RwLock;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, ToolCall, Tool, TodoProcessor};
use anyhow::Result;
use lazy_static::lazy_static;
use anyhow::anyhow;
use async_trait::async_trait;
use crate::ai::AiProvider;

#[cfg(feature = "git-agent")]
pub mod git_assistant;
#[cfg(feature = "git-agent")]
pub use git_assistant::GitAssistantAgent;

#[cfg(feature = "haiku-agent")]
pub mod haiku;
#[cfg(feature = "haiku-agent")]
pub use haiku::HaikuAgent;

#[cfg(feature = "greeter-agent")]
pub mod greeter;
#[cfg(feature = "greeter-agent")]
pub use greeter::GreeterAgent;

#[cfg(feature = "browser-agent")]
pub mod browser;
#[cfg(feature = "browser-agent")]
pub use browser::BrowserAgentWrapper;

#[cfg(feature = "project-agent")]
pub mod project;
#[cfg(feature = "project-agent")]
pub use project::ProjectAgent;

pub mod user_agent;
pub mod transfer;
pub mod wrapper;
#[cfg(feature = "rl")]
pub mod rl;

pub use user_agent::UserAgent;
pub use transfer::TransferService;
pub use wrapper::AgentWrapper;

pub struct AgentRegistry {
    pub agents: HashMap<String, AgentWrapper>,
    current_agent: Option<String>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            current_agent: None,
        }
    }

    pub async fn register(&mut self, name: String, agent: Box<dyn Agent + Send + Sync>) -> Result<()> {
        self.agents.insert(name, AgentWrapper::new(agent));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&AgentWrapper> {
        self.agents.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut AgentWrapper> {
        self.agents.get_mut(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    pub fn get_current_agent(&self) -> Option<&str> {
        self.current_agent.as_deref()
    }

    pub fn set_current_agent(&mut self, agent: String) {
        self.current_agent = Some(agent);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &AgentWrapper)> {
        self.agents.iter()
    }

    pub async fn create_default_agents(configs: Vec<AgentConfig>) -> Result<Self> {
        let mut registry = Self::new();
        for config in configs {
            let agent = create_agent(config.clone()).await?;
            registry.register(config.name, agent).await?;
        }
        Ok(registry)
    }
}

pub async fn create_agent(config: AgentConfig) -> Result<Box<dyn Agent + Send + Sync>> {
    match config.name.as_str() {
        #[cfg(feature = "project-agent")]
        "project" => {
            let agent = ProjectAgent::new(config).await.map_err(|e| anyhow!("{}", e))?;
            Ok(Box::new(agent))
        }
        #[cfg(feature = "git-agent")]
        "git" => {
            let agent = GitAssistantAgent::new(config);
            Ok(Box::new(agent))
        }
        #[cfg(feature = "greeter-agent")]
        "greeter" => {
            let agent = GreeterAgent::new(config);
            Ok(Box::new(agent))
        }
        #[cfg(feature = "haiku-agent")]
        "haiku" => {
            let agent = HaikuAgent::new(config);
            Ok(Box::new(agent))
        }
        #[cfg(feature = "browser-agent")]
        "browser" => {
            let agent = browser::BrowserAgentWrapper::new(config)?;
            Ok(Box::new(agent))
        }
        _ => Err(anyhow!("Unknown agent type: {}", config.name)),
    }
}

lazy_static! {
    pub static ref GLOBAL_REGISTRY: Arc<RwLock<AgentRegistry>> = Arc::new(RwLock::new(AgentRegistry::new()));
}

pub async fn register_agent(agent: Box<dyn Agent + Send + Sync>) -> Result<()> {
    let mut registry = GLOBAL_REGISTRY.write().await;
    let config = agent.get_config().await?;
    registry.register(config.name, agent).await
}

pub async fn get_agent(name: &str) -> Option<Arc<Box<dyn Agent + Send + Sync>>> {
    let registry = GLOBAL_REGISTRY.read().await;
    registry.get(name).map(|wrapper| {
        let boxed: Box<dyn Agent + Send + Sync> = Box::new(wrapper.clone());
        Arc::new(boxed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, State, StateMachine, AgentStateManager};
    use crate::agents::greeter::GreeterAgent;

    fn create_test_configs() -> Vec<AgentConfig> {
        vec![
            AgentConfig {
                name: String::from("greeter"),
                public_description: String::from("Greets users"),
                instructions: String::from("Greet the user"),
                tools: vec![],
                downstream_agents: vec![String::from("haiku")],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: String::from("haiku"),
                public_description: String::from("Creates haikus"),
                instructions: String::from("Create haikus"),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
        ]
    }

    #[tokio::test]
    async fn test_agent_registry() -> Result<()> {
        let configs = create_test_configs();
        let mut registry = AgentRegistry::new();

        // Create a greeter agent directly with a mock AI client
        struct MockAiClient;
        #[async_trait]
        impl AiProvider for MockAiClient {
            async fn chat(&self, _system_prompt: &str, _messages: Vec<HashMap<String, String>>) -> Result<String> {
                Ok("Hello!".to_string())
            }
        }

        let mut greeter = GreeterAgent::new(configs[0].clone());
        greeter = greeter.with_ai_client(MockAiClient);
        registry.register(configs[0].name.clone(), Box::new(greeter)).await?;

        // Test immutable access
        assert!(registry.get("greeter").is_some());
        assert!(registry.get("nonexistent").is_none());

        // Test mutable access
        if let Some(greeter) = registry.get_mut("greeter") {
            let response = greeter.process_message(Message::new(String::from("hi"))).await?;
            assert!(response.content.contains("Hello"));
        }

        // Test agent iteration
        let all_agents: Vec<_> = registry.iter().map(|(k, _)| k).collect();
        assert_eq!(all_agents.len(), 1);
        Ok(())
    }

    #[tokio::test]
    #[cfg(all(feature = "greeter-agent", feature = "haiku-agent"))]
    async fn test_agent_workflow() -> Result<()> {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let service = TransferService::new(registry.clone());

        // Register test agents
        {
            let mut registry = registry.write().await;
            let greeter = GreeterAgent::new(AgentConfig {
                name: "greeter".to_string(),
                public_description: "Test greeter".to_string(),
                instructions: "Test greetings".to_string(),
                tools: vec![],
                downstream_agents: vec!["haiku".to_string()],
                personality: None,
                state_machine: None,
            });
            registry.register("greeter".to_string(), Box::new(greeter)).await?;

            let haiku = HaikuAgent::new(AgentConfig {
                name: "haiku".to_string(),
                public_description: "Test haiku".to_string(),
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
            registry.register("haiku".to_string(), Box::new(haiku)).await?;
        }

        // Set initial agent and test workflow
        service.set_current_agent_name("greeter").await?;

        // Test initial greeting
        let response = service.process_message(Message::new("hello".to_string())).await?;
        assert!(response.content.contains("Hello"), "Response should contain a greeting");

        // Test transfer to haiku agent
        service.transfer("greeter", "haiku", Message::new("test".to_string())).await?;
        assert_eq!(service.get_current_agent_name().await?, "haiku");

        // Test haiku generation
        let response = service.process_message(Message::new("nature".to_string())).await?;
        assert!(response.content.contains("haiku"), "Response should contain a haiku");
        Ok(())
    }

    #[tokio::test]
    #[cfg(all(feature = "greeter-agent", feature = "haiku-agent"))]
    async fn test_agent_transfer() -> Result<()> {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let service = TransferService::new(registry.clone());

        // Set up test agents
        {
            let mut reg = registry.write().await;
            let greeter = GreeterAgent::new(AgentConfig {
                name: "greeter".to_string(),
                public_description: "Test greeter agent".to_string(),
                instructions: "Test greeting".to_string(),
                tools: vec![],
                downstream_agents: vec!["haiku".to_string()],
                personality: None,
                state_machine: None,
            });

            let haiku = HaikuAgent::new(AgentConfig {
                name: "haiku".to_string(),
                public_description: "Test haiku agent".to_string(),
                instructions: "Test haiku generation".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });

            reg.register("greeter".to_string(), Box::new(greeter)).await?;
            reg.register("haiku".to_string(), Box::new(haiku)).await?;
        }

        // Test agent transfer
        service.set_current_agent_name("greeter").await?;
        let message = Message::new("Hello, transfer me to haiku!".to_string());
        service.transfer("greeter", "haiku", message).await?;
        assert_eq!(service.get_current_agent_name().await?, "haiku");
        Ok(())
    }
}

pub fn default_agents() -> Vec<AgentConfig> {
    let mut agents = Vec::new();

    #[cfg(feature = "greeter-agent")]
    agents.push(AgentConfig {
        name: "greeter".to_string(),
        public_description: "Agent that greets the user.".to_string(),
        instructions: "Greet users and make them feel welcome.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "haiku-agent")]
    agents.push(AgentConfig {
        name: "haiku".to_string(),
        public_description: "Agent that creates haikus.".to_string(),
        instructions: "Create haikus based on user input.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "git-agent")]
    agents.push(AgentConfig {
        name: "git".to_string(),
        public_description: "Agent that helps with git operations.".to_string(),
        instructions: "Help users with git operations like commit, branch, merge etc.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "project-init-agent")]
    agents.push(AgentConfig {
        name: "project-init".to_string(),
        public_description: "Agent that helps initialize new projects.".to_string(),
        instructions: "Help users create new projects with proper structure and configuration.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "browser-agent")]
    agents.push(AgentConfig {
        name: "browser".to_string(),
        public_description: "Agent that controls browser automation.".to_string(),
        instructions: "Help users with browser automation tasks.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    agents
}
