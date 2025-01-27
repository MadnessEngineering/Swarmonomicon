use std::collections::HashMap;
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

pub struct UserAgent {
    config: AgentConfig,
    todos: Vec<TodoItem>,
    current_todo: Option<usize>,
}

impl UserAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            todos: Vec::new(),
            current_todo: None,
        }
    }

    pub fn add_todo(&mut self, description: String, context: HashMap<String, String>) {
        self.todos.push(TodoItem {
            description,
            status: TodoStatus::Pending,
            agent: None,
            context,
        });
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
        if let Some(index) = self.current_todo {
            if let Some(todo) = self.todos.get_mut(index) {
                if todo.status == TodoStatus::Pending {
                    // Determine which agent should handle this
                    if let Some(agent_name) = self.determine_next_agent(todo).await? {
                        todo.agent = Some(agent_name.clone());
                        todo.status = TodoStatus::InProgress;
                        Ok(format!("Assigned task to {} agent: {}", agent_name, todo.description))
                    } else {
                        todo.status = TodoStatus::Failed;
                        Ok("No suitable agent found for this task.".to_string())
                    }
                } else {
                    Ok(format!("Current task status: {:?}", todo.status))
                }
            } else {
                Ok("No current task selected.".to_string())
            }
        } else {
            Ok("No current task selected.".to_string())
        }
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
                self.add_todo(description, context);
                "Todo added successfully.".to_string()
            },
            "list" => {
                let mut output = String::new();
                for (i, todo) in self.todos.iter().enumerate() {
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
                if let Some(index) = self.todos.iter().position(|t| t.status == TodoStatus::Pending) {
                    self.current_todo = Some(index);
                    self.process_current_todo().await?
                } else {
                    "No pending todos found.".to_string()
                }
            },
            "status" => {
                if let Some(index) = self.current_todo {
                    if let Some(todo) = self.todos.get(index) {
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
