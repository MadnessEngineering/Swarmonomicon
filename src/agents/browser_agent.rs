use crate::types::{Agent, AgentConfig, Result};
use browser_agent::BrowserAgent as BrowserAgentInner;

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

    async fn process_message(&self, message: &str) -> Result<String> {
        let result = self.inner.process_message(message).await?;
        Ok(result)
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await?;
        Ok(())
    }
}
