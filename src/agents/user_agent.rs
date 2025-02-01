use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::types::{Agent, AgentConfig, Result, Message, Tool, State};
use crate::error::Error;
use std::collections::HashMap;
use async_trait::async_trait;
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub description: String,
    pub status: TodoStatus,
    pub assigned_agent: Option<String>,
    pub context: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserAgentState {
    todos: Vec<TodoItem>,
    last_processed: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct UserAgent {
    config: AgentConfig,
    state: UserAgentState,
    state_file: Option<String>,
}

impl UserAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            state: UserAgentState {
                todos: Vec::new(),
                last_processed: None,
            },
            state_file: None,
        }
    }

    pub fn with_state_file(config: AgentConfig, state_file: impl Into<String>) -> Result<Self> {
        let state_file = state_file.into();
        let state = if Path::new(&state_file).exists() {
            let contents = fs::read_to_string(&state_file)?;
            serde_json::from_str(&contents)?
        } else {
            UserAgentState {
                todos: Vec::new(),
                last_processed: None,
            }
        };

        Ok(Self {
            config,
            state,
            state_file: Some(state_file),
        })
    }

    fn save_state(&self) -> Result<()> {
        if let Some(state_file) = &self.state_file {
            let contents = serde_json::to_string_pretty(&self.state)?;
            fs::write(state_file, contents)?;
        }
        Ok(())
    }

    pub fn add_todo(&mut self, description: String, context: Option<String>) -> Result<()> {
        let todo = TodoItem {
            description,
            status: TodoStatus::Pending,
            assigned_agent: None,
            context,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.state.todos.push(todo);
        self.save_state()?;
        Ok(())
    }

    pub fn get_next_pending_todo(&self) -> Option<(usize, &TodoItem)> {
        self.state.todos.iter().enumerate()
            .find(|(_, todo)| todo.status == TodoStatus::Pending)
    }

    pub fn mark_todo_completed(&mut self, index: usize) -> Result<()> {
        if let Some(todo) = self.state.todos.get_mut(index) {
            todo.status = TodoStatus::Completed;
            todo.updated_at = Utc::now();
            self.save_state()?;
        }
        Ok(())
    }

    pub fn mark_todo_failed(&mut self, index: usize, error: Option<String>) -> Result<()> {
        if let Some(todo) = self.state.todos.get_mut(index) {
            todo.status = TodoStatus::Failed;
            todo.error = error;
            todo.updated_at = Utc::now();
            self.save_state()?;
        }
        Ok(())
    }

    pub fn get_last_processed(&self) -> Option<DateTime<Utc>> {
        self.state.last_processed
    }

    pub fn update_last_processed(&mut self) -> Result<()> {
        self.state.last_processed = Some(Utc::now());
        self.save_state()?;
        Ok(())
    }

    pub async fn determine_next_agent(&self, todo: &TodoItem) -> Result<Option<String>> {
        // Use OpenAI to determine which agent should handle this task
        let prompt = format!(
            "Based on the following task description and context, which agent should handle it?\n\
            Task: {}\n\
            Context: {}\n\n\
            Available agents:\n\
            - git (for git operations and repository management)\n\
            - project-init (for project initialization and setup)\n\
            - haiku (for documentation and creative writing)\n\
            - browser (for browser automation tasks)\n\
            - greeter (for user interaction and routing)\n\
            Respond with just the agent name or 'none' if no agent is suitable.",
            todo.description,
            todo.context.as_deref().unwrap_or("No context provided")
        );

        // TODO: Call OpenAI API to get response
        // For now, return a simple heuristic-based decision
        let description = todo.description.to_lowercase();
        let agent = if description.contains("git") || description.contains("commit") || description.contains("branch") || description.contains("repo") {
            Some("git".to_string())
        } else if description.contains("project") || description.contains("init") || description.contains("create") || description.contains("setup") || description.contains("new") {
            Some("project-init".to_string())
        } else if description.contains("doc") || description.contains("haiku") || description.contains("poem") || description.contains("write") {
            Some("haiku".to_string())
        } else if description.contains("browser") || description.contains("web") || description.contains("page") || description.contains("site") {
            Some("browser".to_string())
        } else if description.contains("hello") || description.contains("hi") || description.contains("greet") || description.contains("welcome") {
            Some("greeter".to_string())
        } else {
            None
        };

        Ok(agent)
    }
}

#[async_trait]
impl Agent for UserAgent {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        Ok(Message::new(format!("User received: {}", message.content)))
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
        Ok(self.config.clone())
    }
}
