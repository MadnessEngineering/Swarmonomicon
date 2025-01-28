use std::collections::HashMap;
use async_trait::async_trait;
use crate::types::{Agent, AgentConfig, Message, Result, Tool};
use browser_agent::Conversation;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BrowserAgentConfig {
    pub instructions: String,
}

pub struct BrowserAgentWrapper {
    inner: Conversation,
    browser_config: BrowserAgentConfig,
    agent_config: AgentConfig,
}

impl BrowserAgentWrapper {
    pub fn new(config: AgentConfig) -> Result<Self> {
        let browser_config = BrowserAgentConfig {
            instructions: config.instructions.clone(),
        };
        let inner = Conversation::new(browser_config.instructions.clone());
        Ok(Self {
            inner,
            browser_config,
            agent_config: config,
        })
    }

    pub async fn shutdown(&self) -> Result<()> {
        // TODO: Implement shutdown logic
        Ok(())
    }
}

#[async_trait]
impl Agent for BrowserAgentWrapper {
    async fn process_message(&self, message: Message) -> Result<Message> {
        Ok(Message::new(format!("Browser received: {}", message.content)))
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
        Ok(self.agent_config.clone())
    }
}
