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
    #[serde(default)]
    pub personality: Option<String>,
    #[serde(default)]
    pub state_machine: Option<StateMachine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, String>,
    #[serde(default)]
    pub is_background: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub states: HashMap<String, State>,
    pub initial_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub prompt: String,
    pub transitions: HashMap<String, String>,
    pub validation: Option<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub pattern: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub role: String,
    pub timestamp: u64,
    pub metadata: Option<MessageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub tool_calls: Option<Vec<ToolCall>>,
    pub state: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub parameters: HashMap<String, String>,
    pub result: Option<String>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn process_message(&mut self, message: &str) -> crate::Result<Message>;
    async fn transfer_to(&mut self, agent_name: &str) -> crate::Result<()>;
    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> crate::Result<String>;
    fn get_current_state(&self) -> Option<&State>;
    fn get_config(&self) -> &AgentConfig;
}

// Implement a basic agent state manager
pub struct AgentStateManager {
    current_state: Option<String>,
    state_machine: Option<StateMachine>,
}

impl AgentStateManager {
    pub fn new(state_machine: Option<StateMachine>) -> Self {
        let current_state = state_machine.as_ref().map(|sm| sm.initial_state.clone());
        Self {
            current_state,
            state_machine,
        }
    }

    pub fn transition(&mut self, event: &str) -> Option<&State> {
        if let (Some(state_machine), Some(current_state)) = (&self.state_machine, &self.current_state) {
            if let Some(current) = state_machine.states.get(current_state) {
                if let Some(next_state) = current.transitions.get(event) {
                    self.current_state = Some(next_state.clone());
                    return state_machine.states.get(next_state);
                }
            }
        }
        None
    }

    pub fn get_current_state(&self) -> Option<&State> {
        if let Some(state_machine) = &self.state_machine {
            self.current_state.as_ref().and_then(|current| state_machine.states.get(current))
        } else {
            None
        }
    }

    pub fn get_current_state_name(&self) -> Option<&str> {
        self.current_state.as_deref()
    }
}

// More types will be added as needed
#[allow(dead_code)]
pub struct Unimplemented; 
