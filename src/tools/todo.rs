use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::StreamExt;
use mongodb::{
    bson::{doc, to_bson},
    Client, Collection,
    options::IndexOptions,
    IndexModel,
};
use std::process::Command;
use crate::tools::ToolExecutor;
use crate::types::{TodoTask, TaskPriority, TaskStatus};
use anyhow::{Result, anyhow};
use serde_json::Value;
use uuid::Uuid;
use regex::Regex;
use crate::ai::{AiProvider, DefaultAiClient};

#[derive(Clone)]
pub struct TodoTool {
    collection: Arc<Collection<TodoTask>>,
    ai_client: Arc<Box<dyn AiProvider + Send + Sync>>,
}

impl TodoTool {
    pub async fn new() -> Result<Self> {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));

        // Create a unique index on the description field
        let index = IndexModel::builder()
            .keys(doc! { "description": 1 })
            .options(Some(IndexOptions::builder().unique(true).build()))
            .build();
        collection.create_index(index, None)
            .await
            .map_err(|e| anyhow!("Failed to create index: {}", e))?;

        Ok(Self {
            collection,
            ai_client: Arc::new(Box::new(DefaultAiClient::new())),
        })
    }

    pub fn with_ai_client<T: AiProvider + Send + Sync + 'static>(mut self, client: T) -> Self {
        self.ai_client = Arc::new(Box::new(client));
        self
    }

    async fn enhance_with_ai(&self, description: &str) -> Result<(String, TaskPriority)> {
        tracing::debug!("Attempting to enhance description with AI: {}", description);

        let system_prompt = "You are a helpful AI assistant that improves todo task descriptions and assigns priorities. \
                           Your responses should be in JSON format with 'description' and 'priority' fields. \
                           Priority should be one of: 'low', 'medium', or 'high'.";

        let messages = vec![HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), format!(
                "Please improve this todo task description and assign a priority: '{}'. \
                 Return only a JSON object with 'description' and 'priority' fields.",
                description
            )),
        ])];

        let ai_response = self.ai_client.chat(system_prompt, messages).await?;
        tracing::debug!("Received AI response: {}", ai_response);

        // Use regex to extract JSON substring
        let json_regex = Regex::new(r#"\{.*\}"#).unwrap();
        let mut json_content = match json_regex.find(&ai_response) {
            Some(json_match) => {
                let json_str = json_match.as_str().to_string();
                tracing::debug!("Found JSON substring: {}", json_str);
                json_str
            }
            None => {
                tracing::warn!("Could not find JSON substring in AI response");
                ai_response.clone()
            }
        };

        tracing::debug!("Attempting to parse JSON content: {}", json_content);

        // Try to parse the JSON response, with a few rounds of cleanup if needed
        let mut parse_attempts = 0;
        let parsed_json = loop {
            match serde_json::from_str::<Value>(&json_content) {
                Ok(parsed) => break Ok(parsed),
                Err(e) => {
                    parse_attempts += 1;
                    if parse_attempts == 1 {
                        // First attempt failed, try removing trailing commas
                        let cleaned_json = json_content.trim_end_matches(',');
                        tracing::debug!("Attempting to parse cleaned JSON: {}", cleaned_json);
                        json_content = cleaned_json.to_string();
                    } else if parse_attempts == 2 {
                        // Second attempt failed, try stripping non-ASCII
                        let cleaned_json = json_content.replace(|c: char| !c.is_ascii(), "");
                        tracing::debug!("Attempting to parse ASCII-only JSON: {}", cleaned_json);
                        json_content = cleaned_json;
                    } else {
                        // Giving up after 2 failed cleanup attempts
                        tracing::error!("Failed to parse JSON after cleanup attempts: {}", e);
                        break Err(e);
                    }
                }
            }
        };

        match parsed_json {
            Ok(enhanced) => {
                let enhanced_desc = enhanced["description"]
                    .as_str()
                    .unwrap_or(description)
                    .to_string();

                let priority = match enhanced["priority"].as_str().unwrap_or("medium") {
                    "high" => TaskPriority::High,
                    "low" => TaskPriority::Low,
                    _ => TaskPriority::Medium,
                };

                tracing::debug!("Successfully enhanced description: {} with priority: {:?}", enhanced_desc, priority);
                Ok((enhanced_desc, priority))
            }
            Err(_) => {
                tracing::debug!("Falling back to original description with medium priority");
                Ok((description.to_string(), TaskPriority::Medium))
            }
        }
    }

    async fn add_todo(&self, description: &str, context: Option<&str>, target_agent: &str) -> Result<String> {
        tracing::debug!("Adding new todo - Description: {}, Context: {:?}, Target Agent: {}", description, context, target_agent);
        let now = Utc::now();

        // Try to enhance the description with AI, fallback to original if enhancement fails
        tracing::debug!("Attempting AI enhancement..");
        let (enhanced_description, priority) = match self.enhance_with_ai(description).await {
            Ok((desc, prio)) => {
                tracing::debug!("AI enhancement successful!");
                (desc, prio)
            },
            Err(e) => {
                tracing::warn!("Failed to enhance todo with AI: {}", e);
                tracing::debug!("Using original description with medium priority");
                (description.to_string(), TaskPriority::Medium)
            }
        };

        tracing::debug!("Creating new TodoTask with description: {}", enhanced_description);
        let new_todo = TodoTask {
            id: Uuid::new_v4().to_string(),
            description: enhanced_description.clone(),
            priority: priority.clone(),
            source_agent: Some("mcp_server".to_string()),
            target_agent: target_agent.to_string(),
            status: TaskStatus::Pending,
            created_at: now.timestamp(),
            completed_at: None,
        };

        tracing::debug!("Attempting to insert todo into database");
        match self.collection.insert_one(new_todo.clone(), None).await {
            Ok(_) => {
                tracing::info!("Successfully inserted todo into database: {}", enhanced_description);
                Ok(format!("Added new todo: {}", enhanced_description))
            },
            Err(e) => {
                tracing::warn!("Failed to insert todo: {}", e);
                // If insertion fails due to duplicate, generate a new ID and try again
                if e.to_string().contains("duplicate key error") {
                    tracing::debug!("Detected duplicate key error, generating new unique ID");
                    let timestamp = now.format("%Y%m%d_%H%M%S");
                    let unique_id = format!("{}_{}", new_todo.id, timestamp);

                    tracing::debug!("Attempting to insert with unique ID: {}", unique_id);
                    let fallback_todo = TodoTask {
                        id: unique_id,
                        description: new_todo.description,
                        priority: new_todo.priority,
                        source_agent: new_todo.source_agent,
                        target_agent: new_todo.target_agent,
                        status: new_todo.status,
                        created_at: new_todo.created_at,
                        completed_at: new_todo.completed_at,
                    };

                    match self.collection.insert_one(fallback_todo.clone(), None).await {
                        Ok(_) => {
                            tracing::info!("Successfully inserted todo with unique ID into database: {}", enhanced_description);
                            Ok(format!("Added new todo: {}", enhanced_description))
                        },
                        Err(e) => {
                            tracing::error!("Failed to insert todo even with unique ID: {}", e);
                            Err(anyhow!("Failed to insert todo even with unique ID: {}", e))
                        }
                    }
                } else {
                    tracing::error!("Failed to insert todo due to non-duplicate error: {}", e);
                    Err(anyhow!("Failed to insert todo: {}", e))
                }
            }
        }
    }

    async fn list_todos(&self) -> Result<String> {
        let mut cursor = self.collection.find(None, None)
            .await
            .map_err(|e| anyhow!("Failed to find todos: {}", e))?;
        let mut todos = Vec::new();

        while let Some(todo_result) = cursor.next().await {
            if let Ok(todo) = todo_result {
                todos.push(todo);
            }
        }

        if todos.is_empty() {
            return Ok("No todos found.".to_string());
        }

        let mut output = String::from("Current todos:\n");
        for todo in todos {
            output.push_str(&format!("- {} ({:?})\n", todo.description, todo.status));
        }

        Ok(output)
    }

    async fn update_todo_status(&self, description: &str, status: TaskStatus) -> Result<String> {
        let now = Utc::now();
        let status_bson = to_bson(&status)
            .map_err(|e| anyhow!("Failed to convert status to BSON: {}", e))?;

        let update_result = self.collection.update_one(
            doc! { "description": description },
            doc! {
                "$set": {
                    "status": status_bson,
                    "updated_at": now.timestamp()
                }
            },
            None,
        )
        .await
        .map_err(|e| anyhow!("Failed to update todo: {}", e))?;

        if update_result.modified_count == 1 {
            Ok(format!("Updated todo status to {:?}", status))
        } else {
            Err(anyhow!("Todo with description '{}' not found", description))
        }
    }
}

#[async_trait]
impl ToolExecutor for TodoTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let command = params.get("command").ok_or_else(|| anyhow!("Missing command parameter"))?;
        tracing::debug!("Executing TodoTool command: {}", command);

        match command.as_str() {
            "add" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                let context = params.get("context").map(|s| s.as_str());
                let default_agent = "user".to_string();
                let target_agent = params.get("target_agent").unwrap_or(&default_agent);
                tracing::debug!("Adding todo - Description: {}, Context: {:?}, Target Agent: {}", description, context, target_agent);
                self.add_todo(description, context, target_agent).await
            }
            "list" => {
                tracing::debug!("Listing todos");
                self.list_todos().await
            }
            "complete" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                tracing::debug!("Marking todo as complete: {}", description);
                self.update_todo_status(description, TaskStatus::Completed).await
            }
            "fail" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                tracing::debug!("Marking todo as failed: {}", description);
                self.update_todo_status(description, TaskStatus::Failed).await
            }
            _ => {
                tracing::error!("Unknown todo command: {}", command);
                Err(anyhow!("Unknown todo command"))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_operations() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));

        let tool = TodoTool { collection };

        // Test adding a todo
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Test todo".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Added new todo"));

        // Test listing todos
        let mut params = HashMap::new();
        params.insert("command".to_string(), "list".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Test todo"));

        // Cleanup: Drop the test collection
        Arc::try_unwrap(tool.collection)
            .unwrap()
            .drop(None)
            .await
            .map_err(|e| anyhow!("Failed to drop test collection: {}", e))?;

        Ok(())
    }
}
