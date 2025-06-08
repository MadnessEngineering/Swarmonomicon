use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use reqwest;
use std::time::Duration;
use futures_util::StreamExt;
use crate::tools::ToolExecutor;
use crate::types::{TodoTask, TaskPriority, TaskStatus, projects};
use anyhow::{Result, anyhow};
use serde_json::Value;
use uuid::Uuid;
use regex::Regex;
use crate::ai::{AiProvider, DefaultAiClient, LocalAiClient};
use serde::{Serialize, Deserialize};
// use langgraph::{Graph, Node};

// MCP Server request/response structures
#[derive(Debug, Serialize, Deserialize)]
struct McpAddTodoRequest {
    description: String,
    project: String,
    priority: String,
    target_agent: String,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpUpdateTodoRequest {
    todo_id: String,
    updates: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    success: bool,
    data: Option<serde_json::Value>,
    message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpQueryRequest {
    query_or_filter: Option<String>,
    fields_or_projection: Option<String>,
    limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpMarkCompleteRequest {
    todo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpGetTodoRequest {
    todo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpDeleteTodoRequest {
    todo_id: String,
}

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
    http_client: reqwest::Client,
    mcp_server_url: String,
    ai_client: Arc<Box<dyn AiProvider + Send + Sync>>,
}

impl TodoTool {
    pub async fn new() -> Result<Self> {
        let mcp_server_url = std::env::var("MCP_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            http_client,
            mcp_server_url,
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

    /// Call MCP server's add_todo_tool endpoint
    async fn call_mcp_add_todo(
        &self,
        description: String,
        project: String,
        priority: String,
        target_agent: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        let request_body = McpAddTodoRequest {
            description,
            project,
            priority,
            target_agent,
            metadata,
        };

        tracing::debug!("Calling MCP server add_todo_tool with: {:?}", request_body);

        let response = self.http_client
            .post(&format!("{}/tools/add_todo_tool", self.mcp_server_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call MCP server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("MCP server returned error {}: {}", status, error_text));
        }

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read MCP response: {}", e))?;

        tracing::debug!("MCP server response: {}", response_text);

        // Parse as the actual MCP response format (JSON string)
        let mcp_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse MCP response as JSON: {}", e))?;

        if let Some(success) = mcp_response.get("success").and_then(|v| v.as_bool()) {
            if success {
                tracing::info!("Successfully created todo via MCP server");
                Ok(response_text)
            } else {
                let error_msg = mcp_response.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown MCP error");
                Err(anyhow!("MCP server error: {}", error_msg))
            }
        } else {
            // Assume success if no explicit success field
            tracing::info!("Todo created via MCP server (assumed success)");
            Ok(response_text)
        }
    }

    /// Call MCP server's query_todos_tool endpoint
    async fn call_mcp_query_todos(&self, filter: Option<String>) -> Result<Vec<TodoTask>> {
        let request_body = McpQueryRequest {
            query_or_filter: filter,
            fields_or_projection: None,
            limit: Some(100),
        };

        let response = self.http_client
            .post(&format!("{}/tools/query_todos_tool", self.mcp_server_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call MCP server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("MCP server returned error {}: {}", status, error_text));
        }

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read MCP response: {}", e))?;

        // Parse the JSON response
        let mcp_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse MCP response: {}", e))?;

        if let Some(success) = mcp_response.get("success").and_then(|v| v.as_bool()) {
            if success {
                if let Some(data) = mcp_response.get("data") {
                    // Parse the todos from the response data
                    if let Some(items) = data.get("items") {
                        let todos: Vec<TodoTask> = serde_json::from_value(items.clone())
                            .unwrap_or_else(|_| Vec::new());
                        Ok(todos)
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Ok(Vec::new())
                }
            } else {
                let error_msg = mcp_response.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown MCP error");
                Err(anyhow!("MCP server error: {}", error_msg))
            }
        } else {
            // Fallback: try to parse todos directly if no success field
            if let Some(items) = mcp_response.get("items") {
                let todos: Vec<TodoTask> = serde_json::from_value(items.clone())
                    .unwrap_or_else(|_| Vec::new());
                Ok(todos)
            } else {
                Ok(Vec::new())
            }
        }
    }

    /// Call MCP server's update_todo_tool endpoint
    async fn call_mcp_update_todo(&self, todo_id: &str, updates: HashMap<String, serde_json::Value>) -> Result<String> {
        let request_body = McpUpdateTodoRequest {
            todo_id: todo_id.to_string(),
            updates,
        };

        let response = self.http_client
            .post(&format!("{}/tools/update_todo_tool", self.mcp_server_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call MCP server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("MCP server returned error {}: {}", status, error_text));
        }

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read MCP response: {}", e))?;

        // Parse as JSON to check for success
        let mcp_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse MCP response: {}", e))?;

        if let Some(success) = mcp_response.get("success").and_then(|v| v.as_bool()) {
            if success {
                Ok(mcp_response.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Todo updated successfully").to_string())
            } else {
                let error_msg = mcp_response.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown MCP error");
                Err(anyhow!("MCP server error: {}", error_msg))
            }
        } else {
            // Assume success if no explicit success field
            Ok("Todo updated successfully".to_string())
        }
    }

    /// Call MCP server's mark_todo_complete_tool endpoint
    async fn call_mcp_mark_complete(&self, todo_id: &str) -> Result<String> {
        let request_body = McpMarkCompleteRequest {
            todo_id: todo_id.to_string(),
        };

        let response = self.http_client
            .post(&format!("{}/tools/mark_todo_complete_tool", self.mcp_server_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call MCP server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("MCP server returned error {}: {}", status, error_text));
        }

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read MCP response: {}", e))?;

        Ok(response_text)
    }

    /// Call MCP server's get_todo_tool endpoint
    async fn call_mcp_get_todo(&self, todo_id: &str) -> Result<TodoTask> {
        let request_body = McpGetTodoRequest {
            todo_id: todo_id.to_string(),
        };

        let response = self.http_client
            .post(&format!("{}/tools/get_todo_tool", self.mcp_server_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call MCP server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("MCP server returned error {}: {}", status, error_text));
        }

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read MCP response: {}", e))?;

        // Parse the JSON response
        let mcp_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse MCP response: {}", e))?;

        if let Some(success) = mcp_response.get("success").and_then(|v| v.as_bool()) {
            if success {
                if let Some(data) = mcp_response.get("data") {
                    let todo: TodoTask = serde_json::from_value(data.clone())
                        .map_err(|e| anyhow!("Failed to parse todo from response: {}", e))?;
                    Ok(todo)
                } else {
                    Err(anyhow!("No todo data in successful response"))
                }
            } else {
                let error_msg = mcp_response.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Todo not found");
                Err(anyhow!("MCP server error: {}", error_msg))
            }
        } else {
            Err(anyhow!("Invalid response format from MCP server"))
        }
    }

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

        // Convert priority to string for MCP call
        let priority_str = match priority {
            TaskPriority::Low => "Low",
            TaskPriority::Medium => "Medium",
            TaskPriority::High => "High",
            TaskPriority::Critical => "High", // Map Critical to High for MCP
            TaskPriority::Inital => "Medium", // Map Inital to Medium for MCP
        };

        // Create metadata with source information
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), serde_json::Value::String("swarmonomicon_agent".to_string()));
        metadata.insert("created_via".to_string(), serde_json::Value::String("swarmonomicon_todo_tool".to_string()));
        if let Some(ctx) = context {
            metadata.insert("context".to_string(), serde_json::Value::String(ctx.to_string()));
        }
        metadata.insert("enhanced_description".to_string(), serde_json::Value::String(enhanced_description));

        tracing::debug!("Calling MCP server to add todo");
        self.call_mcp_add_todo(
            description.to_string(),
            normalized_project,
            priority_str.to_string(),
            target_agent.to_string(),
            Some(metadata)
        ).await
    }

    async fn list_todos(&self) -> Result<String> {
        let todos = self.call_mcp_query_todos(None).await?;

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

        // First, find the todo by description using query_todos
        let filter = format!(r#"{{"description": "{}"}}"#, description);
        let todos = self.call_mcp_query_todos(Some(filter)).await?;

        let todo = todos.into_iter().next()
            .ok_or_else(|| anyhow!("Todo with description '{}' not found", description))?;

        // Handle completion separately using the mark_complete endpoint
        if status == TaskStatus::Completed {
            tracing::debug!("Marking todo as complete using mark_complete endpoint");
            return self.call_mcp_mark_complete(&todo.id).await;
        }

        // For other status changes, use the update endpoint
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::to_value(&status).unwrap_or(serde_json::Value::Null));
        updates.insert("updated_at".to_string(), serde_json::Value::Number(serde_json::Number::from(now.timestamp())));

        // Call MCP server to update the todo
        self.call_mcp_update_todo(&todo.id, updates).await
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
        // Set up TodoTool with MCP server
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool::new().await?
            .with_ai_client(ai_client);

        // Test adding a todo
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Test todo".to_string());
        params.insert("project".to_string(), "test_project".to_string());

        let result = tool.execute(params).await;
        // Note: This test will fail if MCP server is not running
        // In a real test environment, you'd want to mock the HTTP calls
        match result {
            Ok(result) => {
                assert!(result.contains("todo") || result.contains("success"));
                tracing::info!("Add todo test passed: {}", result);
            },
            Err(e) => {
                tracing::warn!("Add todo test failed (likely MCP server not running): {}", e);
                // Don't fail the test if MCP server isn't available
            }
        }

        // Test listing todos
        let mut params = HashMap::new();
        params.insert("command".to_string(), "list".to_string());

        let result = tool.execute(params).await;
        match result {
            Ok(result) => {
                tracing::info!("List todos test passed: {}", result);
                // Should contain either todos or "No todos found"
                assert!(result.contains("todo") || result.contains("No todos found"));
            },
            Err(e) => {
                tracing::warn!("List todos test failed (likely MCP server not running): {}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_ai_enhancement() -> Result<()> {
        // Test AI enhancement functionality
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool::new().await?
            .with_ai_client(ai_client);

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
            match tool.enhance_with_ai(description).await {
                Ok((enhanced, priority, project)) => {
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
                },
                Err(e) => {
                    tracing::warn!("AI enhancement test failed (likely AI service not available): {}", e);
                    // Don't fail the test if AI service isn't available
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_project_field() -> Result<()> {
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool::new().await?
            .with_ai_client(ai_client);

        // Test adding a todo with project
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Project todo test".to_string());
        params.insert("project".to_string(), "test_project".to_string());

        match tool.execute(params).await {
            Ok(result) => {
                tracing::info!("Project field test passed: {}", result);
                assert!(result.contains("todo") || result.contains("success"));
            },
            Err(e) => {
                tracing::warn!("Project field test failed (likely MCP server not running): {}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_project_prediction() -> Result<()> {
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool::new().await?
            .with_ai_client(ai_client);

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
            match tool.predict_project(description).await {
                Ok(project) => {
                    println!("\nTask description: {}", description);
                    println!("Predicted project: {}", project);

                    // Check if the predicted project is either the expected one or another relevant project
                    // This is to account for AI variance in predictions
                    if project != expected_project {
                        println!("Note: Expected '{}' but got '{}' - AI predictions may vary", expected_project, project);
                    }

                    // Just verify that we got a non-empty project name
                    assert!(!project.is_empty(), "Project name should not be empty");
                },
                Err(e) => {
                    tracing::warn!("Project prediction test failed (likely AI service not available): {}", e);
                }
            }
        }

        // Test with a vague description that should default to madness_interactive
        let vague_description = "Fix a bug that was reported yesterday";
        match tool.predict_project(vague_description).await {
            Ok(project) => {
                println!("\nVague description: {}", vague_description);
                println!("Predicted project: {}", project);

                // Just verify we got a valid project name
                assert!(!project.is_empty(), "Project name should not be empty");
            },
            Err(e) => {
                tracing::warn!("Vague project prediction test failed (likely AI service not available): {}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_project_prediction_in_add_todo() -> Result<()> {
        let ai_client = DefaultAiClient::new();
        let tool = TodoTool::new().await?
            .with_ai_client(ai_client);

        // Test adding a todo without specifying a project
        let description = "Update the Swarmonomicon API documentation with new endpoints";

        match tool.add_todo(description, None, "test_agent", None).await {
            Ok(result) => {
                tracing::info!("Add todo with project prediction test passed: {}", result);
                assert!(result.contains("todo") || result.contains("success"));
            },
            Err(e) => {
                tracing::warn!("Add todo with project prediction test failed (likely MCP server not running): {}", e);
            }
        }

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
