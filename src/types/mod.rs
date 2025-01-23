use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub public_description: String,
    pub instructions: String,
    pub tools: Vec<Tool>,
    pub downstream_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, String>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn process_message(&self, message: &str) -> crate::Result<String>;
    async fn transfer_to(&self, agent_name: &str) -> crate::Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub role: String,
    pub timestamp: u64,
}

// More types will be added as needed
#[allow(dead_code)]
pub struct Unimplemented; 
