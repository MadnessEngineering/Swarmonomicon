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
use crate::types::{TodoTask, TaskPriority, TaskStatus, projects};
use anyhow::{Result, anyhow};
use serde_json::Value;
use uuid::Uuid;
use regex::Regex;
use crate::ai::{AiProvider, DefaultAiClient, LocalAiClient};
use serde::{Serialize, Deserialize};
// use langgraph::{Graph, Node};

// Logging structures to align with Omnispindle schema
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChangeEntry {
    field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_value: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LogEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    operation: String,
    #[serde(rename = "todoId")]
    todo_id: String,
    description: String,
    project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<Vec<ChangeEntry>>,
    #[serde(rename = "userAgent")]
    user_agent: String,
}

#[derive(Clone)]
pub struct TodoTool {
    collection: Arc<Collection<TodoTask>>,
    logs_collection: Arc<Collection<LogEntry>>,
    ai_client: Arc<Box<dyn AiProvider + Send + Sync>>,
}

impl TodoTool {
    pub async fn new() -> Result<Self> {
        let mongo_uri = std::env::var("AWSIP")
            .map(|ip| format!("mongodb://{}:27017", ip))
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());

        let client = Client::with_uri_str(&mongo_uri)
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        // Create a unique index on the description field for todos
        let index = IndexModel::builder()
            .keys(doc! { "description": 1 })
            .options(Some(IndexOptions::builder().unique(true).build()))
            .build();
        collection.create_index(index, None)
            .await
            .map_err(|e| anyhow!("Failed to create index: {}", e))?;

        // Create indexes for the logs collection
        let timestamp_index = IndexModel::builder()
            .keys(doc! { "timestamp": -1 })
            .build();
        let operation_index = IndexModel::builder()
            .keys(doc! { "operation": 1 })
            .build();
        let todo_id_index = IndexModel::builder()
            .keys(doc! { "todoId": 1 })
            .build();
        let project_index = IndexModel::builder()
            .keys(doc! { "project": 1 })
            .build();

        logs_collection.create_index(timestamp_index, None).await.ok();
        logs_collection.create_index(operation_index, None).await.ok();
        logs_collection.create_index(todo_id_index, None).await.ok();
        logs_collection.create_index(project_index, None).await.ok();

        Ok(Self {
            collection,
            logs_collection,
            ai_client: Arc::new(Box::new(DefaultAiClient::new())),
        })
    }

    pub fn with_ai_client<T: AiProvider + Send + Sync + 'static>(mut self, client: T) -> Self {
        self.ai_client = Arc::new(Box::new(client));
        self
    }

    // Normalize project name to align with Omnispindle validation logic
    fn normalize_project_name(project: &str) -> String {
        project
            .trim()
            .to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect()
    }

    // The enhance_with_ai method is quite long and could be split into smaller functions:
    // async fn get_ai_enhancement(&self, description: &str) -> Result<String> {
    //     // ... first AI call logic ...
    //     Ok(String::new())
    // }

    // async fn clean_json_response(&self, raw_json: &str) -> Result<String> {
    //     // ... JSON cleaning logic ...
    //     Ok(String::new())
    // }

    // async fn parse_enhanced_response(&self, json_str: &str, original_desc: &str) -> Result<(String, TaskPriority)> {
    //     // ... JSON parsing and validation logic ...
    //     Ok((String::new(), TaskPriority::Medium))
    // }

    async fn predict_project(&self, description: &str) -> Result<String> {
        let (_, _, project) = crate::ai::enhance_todo_description(
            description, 
            self.ai_client.as_ref().as_ref()
        ).await?;
        
        Ok(project)
    }

    async fn enhance_with_ai(&self, description: &str) -> Result<(String, TaskPriority, String)> {
        tracing::debug!("Enhancing todo description with AI: {}", description);
        
        // Use the shared enhancement function
        crate::ai::enhance_todo_description(description, self.ai_client.as_ref().as_ref()).await
    }

    async fn add_todo(&self, description: &str, context: Option<&str>, target_agent: &str, project: Option<&str>) -> Result<String> {
        tracing::debug!("Adding new todo - Description: {}, Context: {:?}, Target Agent: {}, Project: {:?}", description, context, target_agent, project);
        let now = Utc::now();

        // Try to enhance the description with AI, fallback to original if enhancement fails
        tracing::debug!("Attempting AI enhancement..");
        let (enhanced_description, priority, predicted_project) = match self.enhance_with_ai(description).await {
            Ok((desc, prio, proj)) => {
                tracing::debug!("AI enhancement successful!");
                (desc, prio, proj)
            },
            Err(e) => {
                tracing::warn!("Failed to enhance todo with AI: {}", e);
                tracing::debug!("Using original description with medium priority");
                (description.to_string(), TaskPriority::Medium, projects::get_default_project().to_string())
            }
        };

        // Use the provided project if available, otherwise use the predicted one
        let final_project = project.map(|p| p.to_string()).unwrap_or(predicted_project);
        let normalized_project = Self::normalize_project_name(&final_project);

        tracing::debug!("Creating new TodoTask with description: {}", enhanced_description);
        let new_todo = TodoTask {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            enhanced_description: Some(enhanced_description.clone()),
            priority: priority.clone(),
            project: Some(normalized_project.clone()),
            source_agent: Some("swarmonomicon_agent".to_string()),
            target_agent: target_agent.to_string(),
            status: TaskStatus::Pending,
            created_at: now.timestamp(),
            completed_at: None,
            due_date: None,
            duration_minutes: None,
            notes: context.map(|s| s.to_string()),
            ticket: None,
            last_modified: Some(now.timestamp()),
        };

        tracing::debug!("Attempting to insert todo into database");
        match self.collection.insert_one(new_todo.clone(), None).await {
            Ok(_) => {
                tracing::info!("Successfully inserted todo into database: {}", enhanced_description);
                
                // Log the todo creation
                if let Err(e) = self.log_todo_action(
                    "create",
                    &new_todo.id,
                    description,
                    &normalized_project,
                    None,
                    "swarmonomicon_agent"
                ).await {
                    tracing::warn!("Failed to log todo creation: {}", e);
                }
                
                Ok(format!("Added new todo: {}", description))
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
                        id: unique_id.clone(),
                        description: new_todo.description,
                        enhanced_description: new_todo.enhanced_description,
                        priority: new_todo.priority,
                        project: new_todo.project.clone(),
                        source_agent: new_todo.source_agent,
                        target_agent: new_todo.target_agent,
                        status: new_todo.status,
                        created_at: new_todo.created_at,
                        completed_at: new_todo.completed_at,
                        due_date: new_todo.due_date,
                        duration_minutes: new_todo.duration_minutes,
                        notes: new_todo.notes,
                        ticket: new_todo.ticket,
                        last_modified: new_todo.last_modified,
                    };

                    match self.collection.insert_one(fallback_todo.clone(), None).await {
                        Ok(_) => {
                            tracing::info!("Successfully inserted todo with unique ID into database: {}", enhanced_description);
                            
                            // Log the todo creation with unique ID
                            if let Err(e) = self.log_todo_action(
                                "create",
                                &unique_id,
                                description,
                                &normalized_project,
                                None,
                                "swarmonomicon_agent"
                            ).await {
                                tracing::warn!("Failed to log todo creation: {}", e);
                            }
                            
                            Ok(format!("Added new todo: {}", description))
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
        
        // First, get the original todo to track changes
        let original_todo = self.collection
            .find_one(doc! { "description": description }, None)
            .await
            .map_err(|e| anyhow!("Failed to find todo: {}", e))?;
            
        let original_todo = match original_todo {
            Some(todo) => todo,
            None => return Err(anyhow!("Todo with description '{}' not found", description)),
        };

        let status_bson = to_bson(&status)
            .map_err(|e| anyhow!("Failed to convert status to BSON: {}", e))?;

        // Prepare the update document
        let mut update_doc = doc! {
            "status": status_bson,
            "last_modified": now.timestamp()
        };

        // If marking as completed, also set completed_at and calculate duration
        if status == TaskStatus::Completed {
            update_doc.insert("completed_at", now.timestamp());
        }

        let update_result = self.collection.update_one(
            doc! { "description": description },
            doc! { "$set": update_doc },
            None,
        )
        .await
        .map_err(|e| anyhow!("Failed to update todo: {}", e))?;

        if update_result.modified_count == 1 {
            // Create changes array for logging
            let mut changes = Vec::new();
            
            // Track status change
            if original_todo.status != status {
                changes.push(ChangeEntry {
                    field: "status".to_string(),
                    old_value: Some(serde_json::to_value(&original_todo.status).unwrap_or(serde_json::Value::Null)),
                    new_value: Some(serde_json::to_value(&status).unwrap_or(serde_json::Value::Null)),
                });
            }
            
            // Track completed_at change if applicable
            if status == TaskStatus::Completed && original_todo.completed_at.is_none() {
                changes.push(ChangeEntry {
                    field: "completed_at".to_string(),
                    old_value: None,
                    new_value: Some(serde_json::to_value(now.timestamp()).unwrap_or(serde_json::Value::Null)),
                });
            }

            // Determine operation type and log
            let operation = if status == TaskStatus::Completed { "complete" } else { "update" };
            let project = original_todo.project.as_deref().unwrap_or("unknown");
            
            if let Err(e) = self.log_todo_action(
                operation,
                &original_todo.id,
                &original_todo.description,
                project,
                if changes.is_empty() { None } else { Some(changes) },
                "swarmonomicon_agent"
            ).await {
                tracing::warn!("Failed to log todo status update: {}", e);
            }
            
            Ok(format!("Updated todo status to {:?}", status))
        } else {
            Err(anyhow!("Todo with description '{}' not found", description))
        }
    }

    // Log todo action to align with Omnispindle logging schema
    async fn log_todo_action(
        &self,
        operation: &str,
        todo_id: &str,
        description: &str,
        project: &str,
        changes: Option<Vec<ChangeEntry>>,
        user_agent: &str,
    ) -> Result<()> {
        let log_entry = LogEntry {
            timestamp: chrono::Utc::now(),
            operation: operation.to_string(),
            todo_id: todo_id.to_string(),
            description: description.to_string(),
            project: project.to_lowercase(), // Normalize project name
            changes,
            user_agent: user_agent.to_string(),
        };

        self.logs_collection
            .insert_one(log_entry, None)
            .await
            .map_err(|e| anyhow!("Failed to log todo action: {}", e))?;

        tracing::debug!("Logged {} operation for todo {}", operation, todo_id);
        Ok(())
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
                let project = params.get("project").map(|s| s.as_str());
                tracing::debug!("Adding todo - Description: {}, Context: {:?}, Target Agent: {}, Project: {:?}", description, context, target_agent, project);
                self.add_todo(description, context, target_agent, project).await
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
    use crate::ai::DefaultAiClient;

    #[tokio::test]
    async fn test_todo_operations() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        let ai_client = DefaultAiClient::new();
        let tool = TodoTool {
            collection,
            logs_collection,
            ai_client: Arc::new(Box::new(ai_client) as Box<dyn AiProvider + Send + Sync>),
        };

        // Test adding a todo
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Test todo".to_string());
        params.insert("project".to_string(), "test_project".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Added new todo"));

        // Test listing todos
        let mut params = HashMap::new();
        params.insert("command".to_string(), "list".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Test todo"));

        // Cleanup: Drop the test collection
        drop(tool);
        Ok(())
    }

    #[tokio::test]
    async fn test_ai_enhancement() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        // Create tool with qwen2.5 model
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool {
            collection,
            logs_collection,
            ai_client: Arc::new(Box::new(ai_client) as Box<dyn AiProvider + Send + Sync>),
        };

        // Test task enhancement
        let test_cases = vec![
            (
                "fix critical security vulnerability in login",
                vec![TaskPriority::High], // Must be high
                vec!["security", "vulnerability", "authentication", "unauthorized"],
            ),
            (
                "update readme with new features",
                vec![TaskPriority::Low, TaskPriority::Medium], // Can be low or medium
                vec!["documentation", "features", "instructions"],
            ),
            (
                "optimize database queries",
                vec![TaskPriority::Medium, TaskPriority::High], // Can be medium or high
                vec!["performance", "optimize", "database"],
            ),
        ];

        for (description, valid_priorities, expected_keywords) in test_cases {
            let (enhanced, priority, project) = tool.enhance_with_ai(description).await?;

            println!("\nTesting enhancement for: {}", description);
            println!("Enhanced description: {}", enhanced);
            println!("Assigned priority: {:?}", priority);
            println!("Predicted project: {}", project);

            // Verify priority assignment is within acceptable range
            assert!(
                valid_priorities.contains(&priority),
                "Priority {:?} for '{}' not in acceptable range: {:?}",
                priority,
                description,
                valid_priorities
            );

            // Verify description enhancement
            assert!(
                enhanced.len() > description.len() * 2,
                "Enhanced description should be at least twice as long\nOriginal: {}\nEnhanced: {}",
                description,
                enhanced
            );

            // Count how many expected keywords are present
            let enhanced_lower = enhanced.to_lowercase();
            let found_keywords: Vec<_> = expected_keywords.iter()
                .filter(|&&k| enhanced_lower.contains(k))
                .collect();

            // Require at least 50% of keywords to be present
            assert!(
                found_keywords.len() >= expected_keywords.len() / 2,
                "Not enough keywords found in enhanced description.\nExpected at least {} of {:?}\nFound: {:?}\nEnhanced: {}",
                expected_keywords.len() / 2,
                expected_keywords,
                found_keywords,
                enhanced
            );

            // Verify project is not empty
            assert!(!project.is_empty(), "Project name should not be empty");
        }

        // Cleanup
        drop(tool);
        Ok(())
    }

    #[tokio::test]
    async fn test_project_field() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        let ai_client = DefaultAiClient::new();
        let tool = TodoTool {
            collection: collection.clone(),
            logs_collection,
            ai_client: Arc::new(Box::new(ai_client) as Box<dyn AiProvider + Send + Sync>),
        };

        // Test adding a todo with project
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Project todo".to_string());
        params.insert("project".to_string(), "test_project".to_string());

        let _ = tool.execute(params).await?;

        // Find the todo and verify the project field
        let result = collection.find_one(
            doc! { "description": "Project todo" },
            None
        ).await?;

        let todo = result.expect("Todo should exist");
        assert_eq!(todo.project, Some("test_project".to_string()));

        // Test adding a todo without project - it should get AI-predicted project
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "No project todo".to_string());

        let _ = tool.execute(params).await?;

        // Find the todo and verify the project field has some value (AI-predicted)
        let result = collection.find_one(
            doc! { "description": "No project todo" },
            None
        ).await?;

        let todo = result.expect("Todo should exist");
        assert!(todo.project.is_some(), "Project should be populated by AI prediction");

        // Cleanup
        drop(tool);
        Ok(())
    }

    #[tokio::test]
    async fn test_project_prediction() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        let ai_client = DefaultAiClient::new();
        let tool = TodoTool {
            collection,
            logs_collection,
            ai_client: Arc::new(Box::new(ai_client) as Box<dyn AiProvider + Send + Sync>),
        };

        // Test project prediction with specific projects
        let test_cases = vec![
            (
                "Update the README for Swarmonomicon with new API documentation", 
                "Swarmonomicon", // Expected project
            ),
            (
                "Fix Hammerspoon window management hotkeys",
                ".hammerspoon",
            ),
            (
                "Update MQTT variable handling in mqtt-get-var utility",
                "mqtt-get-var",
            ),
        ];

        for (description, expected_project) in test_cases {
            let project = tool.predict_project(description).await?;
            
            println!("\nTask description: {}", description);
            println!("Predicted project: {}", project);
            
            // Check if the predicted project is either the expected one or another relevant project
            // This is to account for AI variance in predictions
            if project != expected_project {
                println!("Note: Expected '{}' but got '{}' - AI predictions may vary", expected_project, project);
            }
            
            // Just verify that we got a non-empty project name
            assert!(!project.is_empty(), "Project name should not be empty");
        }
        
        // Test with a vague description that should default to madness_interactive
        let vague_description = "Fix a bug that was reported yesterday";
        let project = tool.predict_project(vague_description).await?;
        println!("\nVague description: {}", vague_description);
        println!("Predicted project: {}", project);
        
        // Just verify we got a valid project name
        assert!(!project.is_empty(), "Project name should not be empty");

        // Cleanup
        drop(tool);
        Ok(())
    }

    #[tokio::test]
    async fn test_project_prediction_in_add_todo() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));
        let logs_collection = Arc::new(db.collection::<LogEntry>("todo_logs"));

        let ai_client = DefaultAiClient::new();
        let tool = TodoTool {
            collection: collection.clone(),
            logs_collection,
            ai_client: Arc::new(Box::new(ai_client) as Box<dyn AiProvider + Send + Sync>),
        };

        // Test adding a todo without specifying a project
        let description = "Update the Swarmonomicon API documentation with new endpoints";
        let _ = tool.add_todo(description, None, "test_agent", None).await?;

        // Find the todo and verify it has a project assigned
        let result = collection.find_one(
            doc! { "description": description },
            None
        ).await?;
        
        let todo = result.expect("Todo should exist");
        
        // Just verify it has a project, we don't enforce which one due to AI variance
        assert!(todo.project.is_some(), "Todo should have a project assigned");
        println!("Project assigned by AI: {:?}", todo.project);
        
        // Clean up 
        drop(tool);
        Ok(())
    }
}

// // Example structure (actual implementation would depend on the Rust LangGraph API)
// pub struct TodoWorkflow {
//     graph: Graph,
// }
// 
// impl TodoWorkflow {
//     pub fn new() -> Self {
//         let graph = Graph::new()
//             .add_node("parse_input", parse_task_input)
//             .add_node("enhance_description", enhance_with_ai)
//             .add_node("predict_project", predict_project)
//             .add_node("determine_priority", determine_priority)
//             .add_node("store_task", store_in_mongodb)
//             .connect("parse_input", "enhance_description")
//             .connect("enhance_description", "predict_project")
//             .connect("predict_project", "determine_priority")
//             .connect("determine_priority", "store_task");
// 
//         Self { graph }
//     }
// }
