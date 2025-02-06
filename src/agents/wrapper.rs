use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use crate::types::{Agent, Message, Tool, State, AgentConfig, Result};
use crate::types::{TodoProcessor, TodoList, TodoTask};
use futures::executor::block_on;

/// A wrapper type that handles the complexity of agent type management.
/// This provides a consistent interface for working with agents while
/// handling the necessary thread-safety and dynamic dispatch requirements.
#[derive(Clone)]
pub struct AgentWrapper {
    inner: Arc<Box<dyn Agent + Send + Sync>>,
    todo_list: TodoList,
}

impl AgentWrapper {
    /// Create a new AgentWrapper from any type that implements Agent
    pub fn new(agent: Box<dyn Agent + Send + Sync>) -> Self {
        Self {
            inner: Arc::new(agent),
            todo_list: block_on(TodoList::new()).expect("Failed to create TodoList"),
        }
    }
}

#[async_trait]
impl TodoProcessor for AgentWrapper {
    async fn process_task(&self, task: TodoTask) -> Result<Message> {
        // Convert the task to a message and process it
        self.process_message(Message::new(task.description)).await
    }

    fn get_check_interval(&self) -> Duration {
        // Default check interval of 5 seconds
        Duration::from_secs(60)
    }

    fn get_todo_list(&self) -> &TodoList {
        &self.todo_list
    }
}

#[async_trait]
impl Agent for AgentWrapper {
    async fn process_message(&self, message: Message) -> Result<Message> {
        self.inner.process_message(message).await
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        self.inner.transfer_to(target_agent, message).await
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.inner.call_tool(tool, params).await
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        self.inner.get_config().await
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        self.inner.get_current_state().await
    }

    fn get_todo_list(&self) -> Option<&TodoList> {
        Some(&self.todo_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::GreeterAgent;

    #[tokio::test]
    async fn test_agent_wrapper() {
        let config = AgentConfig {
            name: "test".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let agent = GreeterAgent::new(config);
        let wrapper = AgentWrapper::new(Box::new(agent));

        // Test that we can process messages
        let response = wrapper.process_message(Message::new("test".to_string())).await;
        assert!(response.is_ok());

        // Test state access
        let state = wrapper.get_current_state().await;
        assert!(state.is_ok());
    }
}
