use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, Message, Result, AgentConfig, Tool, State, TodoList, TodoTask};
use crate::types::todo::{TodoListExt, TodoProcessor};
use crate::agents::browser_agent::DummyAgent;

/// A wrapper type that handles the complexity of agent type management.
/// This provides a consistent interface for working with agents while
/// handling the necessary thread-safety and dynamic dispatch requirements.
pub struct AgentWrapper {
    pub agent: RwLock<Box<dyn Agent + Send + Sync>>,
    pub todo_list: Arc<RwLock<TodoList>>,
}

impl AgentWrapper {
    /// Create a new AgentWrapper from any type that implements Agent
    pub fn new(agent: Box<dyn Agent + Send + Sync>) -> Self {
        Self {
            agent: RwLock::new(agent),
            todo_list: Arc::new(RwLock::new(TodoList::new())),
        }
    }
}

#[async_trait]
impl Agent for AgentWrapper {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        let mut agent = self.agent.write().await;
        agent.process_message(message).await
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        let agent = self.agent.read().await;
        agent.transfer_to(target_agent, message).await
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        let agent = self.agent.read().await;
        agent.call_tool(tool, params).await
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        let agent = self.agent.read().await;
        agent.get_current_state().await
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        let agent = self.agent.read().await;
        agent.get_config().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_wrapper() {
        let agent = Box::new(DummyAgent::new());
        let mut wrapper = AgentWrapper::new(agent);

        let message = Message::new("test".to_string());
        let response = wrapper.process_message(message).await.unwrap();
        assert!(response.content.contains("test"));
    }
}
