use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use serde_json::Value;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result, ToolCall, Tool};
use crate::types::{TodoProcessor, TodoList, TodoTask};
use crate::ai::AiClient;
use uuid::Uuid;

pub struct GreeterAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
    ai_client: AiClient,
    conversation_history: Vec<Message>,
    todo_list: TodoList,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            state_manager: AgentStateManager::new(None),
            ai_client: AiClient::new(),
            conversation_history: Vec::new(),
            todo_list: TodoList::new(),
        }
    }

    async fn get_ai_response(&self, prompt: &str) -> Result<String> {
        let messages = self.build_conversation_messages(prompt);
        let system_prompt = format!(
            "You are a friendly AI greeter assistant named {}. Your role is to: \
            1. Welcome users and understand their needs \
            2. Direct them to specialized agents for specific tasks (git, haiku, or project initialization) \
            3. Maintain a helpful and engaging conversation \
            4. If the user mentions anything related to git, suggest transferring to the git agent \
            5. If the user mentions poetry or nature, suggest transferring to the haiku agent \
            6. If the user mentions creating or starting a project, suggest transferring to the project-init agent \
            Be concise but friendly in your responses.",
            self.config.name
        );

        self.ai_client.chat(&system_prompt, messages).await
    }

    fn build_conversation_messages(&self, current_prompt: &str) -> Vec<HashMap<String, String>> {
        let mut messages = Vec::new();

        // Add conversation history
        for message in &self.conversation_history {
            messages.push(HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), message.content.clone()),
            ]));
        }

        // Add current prompt
        messages.push(HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), current_prompt.to_string()),
        ]));

        messages
    }

    async fn handle_greeting(&self, message: &str) -> Result<Message> {
        // Check for direct transfer requests first
        let transfer_agent = match message.to_lowercase().as_str() {
            msg if msg.contains("haiku") || msg.contains("poetry") || msg.contains("nature") => Some("haiku"),
            msg if msg.contains("git") || msg.contains("version") || msg.contains("repository") => Some("git"),
            msg if msg.contains("project") || msg.contains("init") || msg.contains("create") => Some("project-init"),
            _ => None,
        };

        if let Some(agent) = transfer_agent {
            let mut response = Message::new(format!("Let me transfer you to our {} specialist...", agent));
            response.metadata = Some(MessageMetadata::new("greeter".to_string())
                .with_transfer(agent.to_string()));
            return Ok(response);
        }

        // Get AI response for conversation
        let ai_response = self.get_ai_response(message).await?;

        let mut response = Message::new(ai_response);
        response.metadata = Some(MessageMetadata::new("greeter".to_string())
            .with_personality(vec!["friendly".to_string(), "helpful".to_string()]));
        Ok(response)
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        self.handle_greeting(&message.content).await
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        // Check if the target agent is in our downstream agents list
        if !self.config.downstream_agents.contains(&target_agent) {
            return Err(format!("Cannot transfer to unknown agent: {}", target_agent).into());
        }

        let mut response = message;
        response.metadata = Some(MessageMetadata::new("greeter".to_string())
            .with_transfer(target_agent));
        Ok(response)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Ok(format!("Called tool {} with params {:?}", tool.name, params))
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(self.state_manager.get_current_state().cloned())
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

#[async_trait]
impl TodoProcessor for GreeterAgent {
    async fn process_task(&self, task: TodoTask) -> Result<Message> {
        // For the greeter, we'll treat tasks as messages to process
        self.process_message(Message::new(task.description)).await
    }

    fn get_check_interval(&self) -> Duration {
        // Check for new tasks every 5 seconds
        Duration::from_secs(5)
    }

    fn get_todo_list(&self) -> &TodoList {
        &self.todo_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Friendly greeter agent".to_string(),
            instructions: "Greet users and direct them to appropriate agents".to_string(),
            tools: vec![],
            downstream_agents: vec![
                "project".to_string(),
                "git".to_string(),
                "haiku".to_string(),
            ],
            personality: Some(serde_json::json!({
                "style": "friendly_receptionist",
                "traits": ["friendly", "helpful", "welcoming"],
                "voice": {
                    "tone": "warm_and_professional",
                    "pacing": "measured",
                    "quirks": ["uses_emojis", "enthusiastic_greetings"]
                }
            }).to_string()),
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeting() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("hi".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.agent, "greeter");
            assert!(metadata.personality_traits.is_some());
        }
    }

    #[tokio::test]
    async fn test_project_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let message = Message::new("I want to create a new project".to_string());
        let response = agent.process_message(message).await.unwrap();
        assert!(response.content.contains("project"));
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("project-init".to_string()));
        }
    }

    #[tokio::test]
    async fn test_git_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("git".to_string())).await.unwrap();
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("git".to_string()));
        }
    }

    #[tokio::test]
    async fn test_haiku_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let response = agent.process_message(Message::new("haiku".to_string())).await.unwrap();
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target, Some("haiku".to_string()));
        }
    }

    #[tokio::test]
    async fn test_invalid_transfer() {
        let agent = GreeterAgent::new(create_test_config());
        let result = agent.transfer_to("nonexistent".to_string(), Message::new("test".to_string())).await;
        assert!(result.is_err(), "Transfer to nonexistent agent should fail");
    }

    #[tokio::test]
    async fn test_todo_processing() {
        let agent = GreeterAgent::new(create_test_config());
        
        // Create a test task
        let task = TodoTask {
            id: Uuid::new_v4().to_string(),
            description: "Hello, I need help with git".to_string(),
            priority: crate::types::TaskPriority::Medium,
            source_agent: None,
            target_agent: "greeter".to_string(),
            status: crate::types::TaskStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
        };

        // Add task to todo list
        <GreeterAgent as TodoProcessor>::get_todo_list(&agent).add_task(task.clone()).await;

        // Process the task
        let response = agent.process_task(task).await.unwrap();

        // Since the message mentions git, it should suggest transferring to the git agent
        assert!(response.metadata.is_some());
        if let Some(metadata) = response.metadata {
            assert_eq!(metadata.transfer_target.unwrap(), "git");
        }
    }
}
