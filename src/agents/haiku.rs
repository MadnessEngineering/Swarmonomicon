use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use serde_json::Value;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result, ToolCall, Tool};
use crate::types::{TodoProcessor, TodoList, TodoTask};
use crate::ai::AiClient;
use uuid::Uuid;
use tokio::sync::RwLock;
use crate::types::todo::TodoListExt;
use chrono::{Utc, DateTime};

pub struct HaikuAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
    ai_client: AiClient,
    conversation_history: Vec<Message>,
    todo_list: Arc<RwLock<TodoList>>,
    state: Option<State>,
}

impl HaikuAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            state_manager: AgentStateManager::new(None),
            ai_client: AiClient::new(),
            conversation_history: Vec::new(),
            todo_list: Arc::new(RwLock::new(TodoList::new())),
            state: None,
        }
    }

    async fn get_ai_response(&self, prompt: &str) -> Result<String> {
        let messages = self.build_conversation_messages(prompt);
        let system_prompt = format!(
            "You are a haiku poet named {}. Your role is to: \
            1. Engage in conversation about nature and poetry \
            2. Write haikus based on topics provided by the user \
            3. Provide feedback and suggestions on haikus written by the user \
            4. Maintain a friendly and creative persona \
            Your haikus should follow the traditional 5-7-5 syllable structure. \
            Focus on themes of nature, seasons, emotions, and beauty.",
            self.config.name
        );

        self.ai_client.chat(&system_prompt, messages).await
    }

    fn build_conversation_messages(&self, current_prompt: &str) -> Vec<HashMap<String, String>> {
        let mut messages = Vec::new();
        for message in &self.conversation_history {
            messages.push(HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), message.content.clone()),
            ]));
        }
        messages.push(HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), current_prompt.to_string()),
        ]));
        messages
    }

    async fn handle_haiku_request(&self, message: &str) -> Result<Message> {
        let ai_response = self.get_ai_response(message).await?;
        let mut response = Message::new(ai_response);
        response.metadata = Some(MessageMetadata::new("haiku".to_string())
            .with_personality(vec!["creative".to_string(), "nature-loving".to_string()]));
        Ok(response)
    }

    pub fn get_todo_list(&self) -> &Arc<RwLock<TodoList>> {
        &self.todo_list
    }
}

#[async_trait]
impl TodoProcessor for HaikuAgent {
    async fn process_task(&mut self, task: TodoTask) -> Result<Message> {
        self.process_message(Message::new(task.description)).await
    }

    async fn get_todo_list(&self) -> Arc<RwLock<TodoList>> {
        self.todo_list.clone()
    }

    async fn start_processing(&mut self) {
        loop {
            let todo_list = TodoProcessor::get_todo_list(self).await;
            let mut list = todo_list.write().await;
            
            if let Some(task) = list.get_next_task() {
                let task_id = task.id.clone();
                drop(list); // Release the lock before processing
                let result = self.process_task(task).await;
                let mut list = todo_list.write().await;
                if result.is_ok() {
                    list.mark_task_completed(&task_id);
                } else {
                    list.mark_task_failed(&task_id);
                }
            } else {
                drop(list);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    fn get_check_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

#[async_trait]
impl Agent for HaikuAgent {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        // Generate a haiku response
        self.handle_haiku_request(&message.content).await
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Err("Transfer not supported by HaikuAgent".into())
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Err("Tool calls not supported by HaikuAgent".into())
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(None)
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }

    fn get_todo_list(&self) -> Option<&TodoList> {
        None // Since we implement TodoProcessor separately
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, State, StateMachine, AgentStateManager};

    #[tokio::test]
    async fn test_haiku_feedback() {
        let mut agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus".to_string(),
            instructions: "Generate haikus".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        let response = agent.process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("haiku"), "Response should contain a haiku");
    }

    #[tokio::test]
    async fn test_invalid_transfer() {
        let agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus".to_string(),
            instructions: "Generate haikus".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        let result = agent.transfer_to("invalid".to_string(), Message::new("test".to_string())).await;
        assert!(result.is_ok(), "Invalid transfer should not panic");
    }

    #[tokio::test]
    async fn test_todo_processing() {
        let mut agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus".to_string(),
            instructions: "Generate haikus".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        let response = agent.process_message(Message::new("todo: write haiku".to_string())).await.unwrap();
        assert!(response.content.contains("haiku"), "Response should contain a haiku");
    }
}
