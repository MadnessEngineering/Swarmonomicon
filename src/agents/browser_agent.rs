use crate::types::{Agent, AgentConfig, Result, Message, Tool, State};
use browser_agent::BrowserAgent as BrowserAgentInner;
use std::collections::HashMap;
use crate::agents::BrowserAgent;

pub struct BrowserAgent {
    inner: BrowserAgentInner,
    config: AgentConfig,
}

impl BrowserAgent {
    pub fn new(config: AgentConfig) -> Self {
        let inner = BrowserAgentInner::new(config.instructions.clone()).expect("Failed to create BrowserAgent");
        Self { inner, config }
    }
}

impl Agent for BrowserAgent {
    fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process_message(&mut self, message: &str) -> Result<Message> {
        let result = self.inner.process_message(message).await?;
        Ok(Message::text(result))
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await?;
        Ok(())
    }

    async fn transfer_to(&mut self, _agent_name: &str) -> Result<()> {
        // TODO: Implement transfer logic
        Ok(())
    }

    async fn call_tool(&mut self, _tool: &Tool, _params: HashMap<String, String>) -> Result<String> {
        // TODO: Implement tool calling logic
        Ok("".to_string())
    }

    fn get_current_state(&self) -> Option<&State> {
        // TODO: Return current state
        None
    }
}
