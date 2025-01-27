use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, Tool};
use crate::Result;

#[derive(Debug, Serialize, Deserialize)]
struct TodoItem {
    description: String,
    status: TodoStatus,
    agent: Option<String>,
    context: HashMap<String, String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
struct TodoState {
    todos: Vec<TodoItem>,
    current_todo: Option<usize>,
    last_processed: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct UserAgent {
    config: AgentConfig,
    state: TodoState,
    state_file: PathBuf,
}

impl UserAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_file = PathBuf::from("todo_state.json");
        let state = Self::load_state(&state_file).unwrap_or_else(|_| TodoState {
            todos: Vec::new(),
            current_todo: None,
            last_processed: None,
        });

        Self {
            config,
            state,
            state_file,
        }
    }

    pub fn with_state_file<P: AsRef<Path>>(config: AgentConfig, state_file: P) -> Self {
        let state_file = state_file.as_ref().to_path_buf();
        let state = Self::load_state(&state_file).unwrap_or_else(|_| TodoState {
            todos: Vec::new(),
            current_todo: None,
            last_processed: None,
        });

        Self {
            config,
            state,
            state_file,
        }
    }

    fn load_state(path: &Path) -> Result<TodoState> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Err("State file does not exist".into())
        }
    }

    fn save_state(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_file, content)?;
        Ok(())
    }

    pub fn add_todo(&mut self, description: String, context: HashMap<String, String>) -> Result<()> {
        let now = chrono::Utc::now();
        self.state.todos.push(TodoItem {
            description,
            status: TodoStatus::Pending,
            agent: None,
            context,
            created_at: now,
            updated_at: now,
        });
        self.save_state()
    }

    pub fn get_next_pending_todo(&self) -> Option<(usize, &TodoItem)> {
        self.state.todos.iter()
            .enumerate()
            .find(|(_, todo)| todo.status == TodoStatus::Pending)
    }

    pub fn mark_todo_completed(&mut self, index: usize) -> Result<()> {
        if let Some(todo) = self.state.todos.get_mut(index) {
            todo.status = TodoStatus::Completed;
            todo.updated_at = chrono::Utc::now();
            self.save_state()?;
        }
        Ok(())
    }

    pub fn mark_todo_failed(&mut self, index: usize, error: Option<String>) -> Result<()> {
        if let Some(todo) = self.state.todos.get_mut(index) {
            todo.status = TodoStatus::Failed;
            todo.updated_at = chrono::Utc::now();
            if let Some(error) = error {
                todo.context.insert("error".to_string(), error);
            }
            self.save_state()?;
        }
        Ok(())
    }

    async fn determine_next_agent(&self, todo: &TodoItem) -> Result<Option<String>> {
        // Use OpenAI to determine which agent should handle this todo
        let openai = reqwest::Client::new();
        let response = openai
            .post("http://127.0.0.1:1234/v1/chat/completions")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "qwen2.5-7b-instruct",
                "messages": [
                    {
                        "role": "system",
                        "content": format!(
                            "You are a task coordinator that determines which agent should handle a given task.\n\
                            Available agents:\n\
                            - git: Handles version control tasks (commits, branches, merges)\n\
                            - project: Initializes and sets up new projects\n\
                            - haiku: Creates haiku-style documentation\n\n\
                            Context variables: {:#?}\n\
                            Respond ONLY with the agent name or UNKNOWN if no agent can handle it.",
                            todo.context
                        )
                    },
                    {
                        "role": "user",
                        "content": format!("Which agent should handle this task: {}", todo.description)
                    }
                ]
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let agent = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string();

        if agent == "UNKNOWN" {
            Ok(None)
        } else {
            Ok(Some(agent))
        }
    }

    async fn process_current_todo(&mut self) -> Result<String> {
        if let Some(index) = self.state.current_todo {
            if let Some(todo) = self.state.todos.get_mut(index) {
                if todo.status == TodoStatus::Pending {
                    // Determine which agent should handle this
                    if let Some(agent_name) = self.determine_next_agent(todo).await? {
                        todo.agent = Some(agent_name.clone());
                        todo.status = TodoStatus::InProgress;
                        todo.updated_at = chrono::Utc::now();
                        self.save_state()?;
                        Ok(format!("Assigned task to {} agent: {}", agent_name, todo.description))
                    } else {
                        self.mark_todo_failed(index, Some("No suitable agent found".to_string()))?;
                        Ok("No suitable agent found for this task.".to_string())
                    }
                } else {
                    Ok(format!("Current task status: {:?} (Last updated: {})",
                        todo.status,
                        todo.updated_at.format("%Y-%m-%d %H:%M:%S")))
                }
            } else {
                Ok("No current task selected.".to_string())
            }
        } else {
            Ok("No current task selected.".to_string())
        }
    }

    pub fn update_last_processed(&mut self) -> Result<()> {
        self.state.last_processed = Some(chrono::Utc::now());
        self.save_state()
    }

    pub fn get_last_processed(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.state.last_processed
    }
}

#[async_trait]
impl Agent for UserAgent {
    async fn process_message(&mut self, message: &str) -> Result<Message> {
        let parts: Vec<&str> = message.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(Message {
                content: "Available commands: add <todo>, list, next, status".to_string(),
                role: "assistant".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                metadata: None,
            });
        }

        let result = match parts[0] {
            "add" if parts.len() > 1 => {
                let description = parts[1..].join(" ");
                let mut context = HashMap::new();
                // You could parse additional context from the description or message
                // For now, we'll just add a timestamp
                context.insert("timestamp".to_string(), chrono::Utc::now().to_string());
                self.add_todo(description, context)?;
                "Todo added successfully.".to_string()
            },
            "list" => {
                let mut output = String::new();
                for (i, todo) in self.state.todos.iter().enumerate() {
                    output.push_str(&format!("{}. [{:?}] {}{}\n",
                        i + 1,
                        todo.status,
                        todo.description,
                        todo.agent.as_ref().map(|a| format!(" (assigned to {})", a)).unwrap_or_default()
                    ));
                }
                if output.is_empty() {
                    "No todos found.".to_string()
                } else {
                    output
                }
            },
            "next" => {
                // Find the next pending todo
                if let Some(index) = self.state.todos.iter().position(|t| t.status == TodoStatus::Pending) {
                    self.state.current_todo = Some(index);
                    self.process_current_todo().await?
                } else {
                    "No pending todos found.".to_string()
                }
            },
            "status" => {
                if let Some(index) = self.state.current_todo {
                    if let Some(todo) = self.state.todos.get(index) {
                        format!("Current task: {} (Status: {:?}{})",
                            todo.description,
                            todo.status,
                            todo.agent.as_ref().map(|a| format!(" - Assigned to {}", a)).unwrap_or_default()
                        )
                    } else {
                        "No current task selected.".to_string()
                    }
                } else {
                    "No current task selected.".to_string()
                }
            },
            _ => "Available commands: add <todo>, list, next, status".to_string(),
        };

        Ok(Message {
            content: result,
            role: "assistant".to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            metadata: None,
        })
    }

    async fn transfer_to(&mut self, _agent_name: &str) -> Result<()> {
        Ok(())
    }

    async fn call_tool(&mut self, _tool: &Tool, _params: HashMap<String, String>) -> Result<String> {
        Ok("Tool execution not implemented for UserAgent".to_string())
    }

    fn get_current_state(&self) -> Option<&State> {
        None
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}
