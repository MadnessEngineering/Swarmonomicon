use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::types::{Agent, AgentConfig, Result, Message, Tool, State};
use crate::error::Error;
use std::collections::HashMap;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAgentState {
    todos: Vec<TodoItem>,
    last_processed: Option<DateTime<Utc>>,
}

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
            Available agents: git (for git operations), project (for project initialization), haiku (for documentation)\n\
            Respond with just the agent name or 'none' if no agent is suitable.",
            todo.description,
            todo.context.as_deref().unwrap_or("No context provided")
        );

        // TODO: Call OpenAI API to get response
        // For now, return a simple heuristic-based decision
        let description = todo.description.to_lowercase();
        let agent = if description.contains("git") || description.contains("commit") || description.contains("branch") {
            Some("git".to_string())
        } else if description.contains("project") || description.contains("init") || description.contains("create") {
            Some("project".to_string())
        } else if description.contains("doc") || description.contains("haiku") {
            Some("haiku".to_string())
        } else {
            None
        };

        Ok(agent)
    }
}

#[async_trait::async_trait]
impl Agent for UserAgent {
    async fn process_message(&mut self, message: Message) -> Result<Message> {
        // Parse commands from the message
        let parts: Vec<&str> = message.content.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(Message::new("Please provide a command".to_string()));
        }

        let response = match parts[0] {
            "add" => {
                let description = parts[1..].join(" ");
                self.add_todo(description, None)?;
                "Todo added successfully".to_string()
            }
            "list" => {
                let mut response = String::new();
                for (i, todo) in self.state.todos.iter().enumerate() {
                    response.push_str(&format!(
                        "{}. [{}] {}\n",
                        i + 1,
                        match todo.status {
                            TodoStatus::Pending => "PENDING",
                            TodoStatus::InProgress => "IN PROGRESS",
                            TodoStatus::Completed => "COMPLETED",
                            TodoStatus::Failed => "FAILED",
                        },
                        todo.description
                    ));
                }
                if response.is_empty() {
                    "No todos found".to_string()
                } else {
                    response
                }
            }
            "process" => {
                if let Some((index, todo)) = self.get_next_pending_todo() {
                    if let Ok(Some(agent)) = self.determine_next_agent(todo).await {
                        format!("Assigning todo to agent: {}", agent)
                    } else {
                        "Could not determine appropriate agent".to_string()
                    }
                } else {
                    "No pending todos found".to_string()
                }
            }
            _ => "Unknown command. Available commands: add, list, process".to_string(),
        };

        Ok(Message::new(response))
    }

    async fn transfer_to(&mut self, _target_agent: String, _message: Message) -> Result<Message> {
        // User agent doesn't transfer to other agents
        Err("UserAgent does not support transfers".into())
    }

    async fn call_tool(&mut self, _tool: &Tool, _params: HashMap<String, String>) -> Result<String> {
        // User agent doesn't use tools directly
        Err("UserAgent does not support direct tool usage".into())
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        // User agent doesn't use state machine
        Ok(None)
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}
