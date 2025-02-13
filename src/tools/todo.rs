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
use crate::agents::user_agent::{TodoItem, TodoStatus};
use anyhow::{Result, anyhow};
use serde_json::Value;

#[derive(Clone)]
pub struct TodoTool {
    collection: Arc<Collection<TodoItem>>,
}

impl TodoTool {
    pub async fn new() -> Result<Self> {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon");
        let collection = Arc::new(db.collection::<TodoItem>("todos"));

        // Create a unique index on the description field
        let index = IndexModel::builder()
            .keys(doc! { "description": 1 })
            .options(Some(IndexOptions::builder().unique(true).build()))
            .build();
        collection.create_index(index, None)
            .await
            .map_err(|e| anyhow!("Failed to create index: {}", e))?;

        Ok(Self { collection })
    }

    async fn enhance_with_ai(&self, description: &str) -> Result<String> {
        // Use goose CLI to enhance the todo description
        let output = Command::new("goose")
            .arg("chat")
            .arg("-m")
            .arg(format!(
                "Given this todo task: '{}', please analyze it and return a JSON object with an enhanced description with more details if possible. \
                 Format: {{\"description\": \"enhanced text\"}}",
                description
            ))
            .output()
            .map_err(|e| anyhow!("Failed to execute goose command: {}", e))?;

        let ai_response = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse goose output: {}", e))?;
        
        // Try to parse the JSON response
        match serde_json::from_str::<Value>(&ai_response) {
            Ok(enhanced) => {
                Ok(enhanced["description"]
                    .as_str()
                    .unwrap_or(description)
                    .to_string())
            }
            Err(_) => {
                // If JSON parsing fails, return original description
                Ok(description.to_string())
            }
        }
    }

    async fn add_todo(&self, description: &str, context: Option<&str>) -> Result<String> {
        let now = Utc::now();
        
        // Try to enhance the description with AI, fallback to original if enhancement fails
        let enhanced_description = match self.enhance_with_ai(description).await {
            Ok(enhanced) => enhanced,
            Err(e) => {
                tracing::warn!("Failed to enhance todo with AI: {}", e);
                description.to_string()
            }
        };

        let new_todo = TodoItem {
            description: enhanced_description.clone(),
            status: TodoStatus::Pending,
            assigned_agent: None,
            context: context.map(|s| s.to_string()),
            error: None,
            created_at: now,
            updated_at: now,
        };

        match self.collection.insert_one(new_todo, None).await {
            Ok(_) => Ok(format!("Added new todo: {}", enhanced_description)),
            Err(e) => {
                // If insertion fails due to duplicate, try to insert with original description
                if e.to_string().contains("duplicate key error") {
                    let fallback_todo = TodoItem {
                        description: description.to_string(),
                        status: TodoStatus::Pending,
                        assigned_agent: None,
                        context: context.map(|s| s.to_string()),
                        error: None,
                        created_at: now,
                        updated_at: now,
                    };
                    self.collection.insert_one(fallback_todo, None)
                        .await
                        .map_err(|e| anyhow!("Failed to insert todo with original description: {}", e))?;
                    Ok(format!("Added new todo: {}", description))
                } else {
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

    async fn update_todo_status(&self, description: &str, status: TodoStatus) -> Result<String> {
        let now = Utc::now();
        let status_bson = to_bson(&status)
            .map_err(|e| anyhow!("Failed to convert status to BSON: {}", e))?;
        
        let update_result = self.collection.update_one(
            doc! { "description": description },
            doc! { 
                "$set": { 
                    "status": status_bson,
                    "updated_at": now 
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

        match command.as_str() {
            "add" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                let context = params.get("context").map(|s| s.as_str());
                self.add_todo(description, context).await
            }
            "list" => {
                self.list_todos().await
            }
            "complete" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                self.update_todo_status(description, TodoStatus::Completed).await
            }
            "fail" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                self.update_todo_status(description, TodoStatus::Failed).await
            }
            _ => Err(anyhow!("Unknown todo command")),
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
        let collection = Arc::new(db.collection::<TodoItem>("todos"));

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
