use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::time;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use crate::tools::ToolExecutor;

// Response schema from MCP Todo Server
#[derive(Serialize, Deserialize, Debug, Clone)]
struct McpResponse {
    status: String,
    message: Option<String>,
    timestamp: String,
    #[serde(default)]
    data: serde_json::Value,
}

// Todo item schema
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpTodo {
    pub id: String,
    pub description: String,
    pub project: String,
    pub priority: String,
    pub status: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Clone)]
pub struct McpTodoClient {
    client: Arc<AsyncClient>,
    requests: Arc<Mutex<HashMap<String, oneshot::Sender<Result<McpResponse>>>>>,
    connected: Arc<Mutex<bool>>,
}

impl McpTodoClient {
    pub async fn new() -> Result<Self> {
        let mqtt_host = env::var("MQTT_HOST").unwrap_or_else(|_| "localhost".to_string());
        let mqtt_port = env::var("MQTT_PORT")
            .unwrap_or_else(|_| "1883".to_string())
            .parse::<u16>()
            .unwrap_or(1883);
            
        let hostname = format!("swarmonomicon-{}", Uuid::new_v4().to_string());
        let mut mqtt_options = MqttOptions::new(hostname, mqtt_host, mqtt_port);
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        mqtt_options.set_clean_session(true);
        
        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
        let requests: Arc<Mutex<HashMap<String, oneshot::Sender<Result<McpResponse>>>>> = 
            Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(Mutex::new(false));
        
        let requests_clone = requests.clone();
        let connected_clone = connected.clone();
        
        // Start background task to handle MQTT events
        tokio::spawn(async move {
            // Subscribe to response topics
            if let Err(e) = client.subscribe("mcp/+/response/#", QoS::AtMostOnce).await {
                tracing::error!("Failed to subscribe to responses: {}", e);
            }
            if let Err(e) = client.subscribe("mcp/+/error/#", QoS::AtMostOnce).await {
                tracing::error!("Failed to subscribe to errors: {}", e);
            }
            
            while let Ok(notification) = eventloop.poll().await {
                match notification {
                    rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) => {
                        let topic = publish.topic.clone();
                        
                        // Check if this is a response to a request
                        if let Some(request_id) = topic.split('/').last() {
                            if let Ok(payload) = std::str::from_utf8(&publish.payload) {
                                // Try to parse as McpResponse
                                if let Ok(response) = serde_json::from_str::<McpResponse>(payload) {
                                    let mut requests = requests_clone.lock().await;
                                    if let Some(sender) = requests.remove(request_id) {
                                        if topic.contains("/error/") {
                                            let _ = sender.send(Err(anyhow!(response.message.unwrap_or_else(|| 
                                                "Unknown error from MCP Todo Server".to_string()))));
                                        } else {
                                            let _ = sender.send(Ok(response));
                                        }
                                    }
                                }
                            }
                        }
                    },
                    rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_)) => {
                        tracing::info!("Connected to MQTT broker");
                        let mut connected = connected_clone.lock().await;
                        *connected = true;
                    },
                    rumqttc::Event::Outgoing(_) => {},
                    e => {
                        tracing::debug!("MQTT Event: {:?}", e);
                    }
                }
            }
            
            tracing::warn!("MCP Todo Client MQTT event loop exited");
            *connected_clone.lock().await = false;
        });
        
        // Wait for connection
        for _ in 0..10 {
            if *connected.lock().await {
                break;
            }
            time::sleep(Duration::from_millis(100)).await;
        }
        
        if !*connected.lock().await {
            return Err(anyhow!("Failed to connect to MQTT broker"));
        }
        
        Ok(Self {
            client: Arc::new(client),
            requests,
            connected,
        })
    }
    
    async fn send_request(&self, target_agent: &str, request: serde_json::Value) -> Result<McpResponse> {
        // Generate unique request ID
        let request_id = Uuid::new_v4().to_string();
        
        // Create response channel
        let (tx, rx) = oneshot::channel();
        
        // Register request
        {
            let mut requests = self.requests.lock().await;
            requests.insert(request_id.clone(), tx);
        }
        
        // Add request ID to payload
        let mut payload = request.as_object().unwrap().clone();
        payload.insert("request_id".to_string(), json!(request_id));
        
        // Publish request
        let topic = format!("mcp/{}/request/{}", target_agent, request_id);
        self.client.publish(topic, QoS::AtLeastOnce, false, serde_json::to_string(&payload)?).await?;
        
        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(10), rx).await {
            Ok(response) => {
                match response {
                    Ok(result) => result,
                    Err(_) => Err(anyhow!("Response channel closed"))
                }
            },
            Err(_) => {
                // Remove request on timeout
                let mut requests = self.requests.lock().await;
                requests.remove(&request_id);
                Err(anyhow!("Request timed out"))
            }
        }
    }
    
    pub async fn add_todo(&self, description: &str, project: &str, priority: Option<&str>, target_agent: Option<&str>) -> Result<String> {
        let payload = json!({
            "command": "add_todo",
            "description": description,
            "project": project,
            "priority": priority.unwrap_or("initial"),
            "target_agent": target_agent.unwrap_or("user"),
        });
        
        let response = self.send_request("todo-server", payload).await?;
        
        // Extract todo_id from the response
        if let Some(data) = response.data.as_object() {
            if let Some(todo_id) = data.get("todo_id") {
                if let Some(id) = todo_id.as_str() {
                    return Ok(id.to_string());
                }
            }
        }
        
        Ok(response.message.unwrap_or_else(|| "Todo added successfully".to_string()))
    }
    
    pub async fn query_todos(&self, filter: Option<serde_json::Value>, limit: Option<u32>) -> Result<Vec<McpTodo>> {
        let payload = json!({
            "command": "query_todos",
            "filter": filter.unwrap_or_else(|| json!({})),
            "limit": limit.unwrap_or(10),
        });
        
        let response = self.send_request("todo-server", payload).await?;
        
        // Parse response data
        if let Some(data) = response.data.as_object() {
            if let Some(items) = data.get("items") {
                if let Some(items_array) = items.as_array() {
                    let mut todos = Vec::new();
                    for item in items_array {
                        if let Ok(todo) = serde_json::from_value::<McpTodo>(item.clone()) {
                            todos.push(todo);
                        }
                    }
                    return Ok(todos);
                }
            }
        }
        
        Err(anyhow!("Failed to parse todos from response"))
    }
    
    pub async fn get_todo(&self, todo_id: &str) -> Result<McpTodo> {
        let payload = json!({
            "command": "get_todo",
            "todo_id": todo_id,
        });
        
        let response = self.send_request("todo-server", payload).await?;
        
        // Parse response data
        if let Ok(todo) = serde_json::from_value::<McpTodo>(response.data) {
            return Ok(todo);
        }
        
        Err(anyhow!("Failed to parse todo from response"))
    }
    
    pub async fn update_todo(&self, todo_id: &str, updates: serde_json::Value) -> Result<String> {
        let payload = json!({
            "command": "update_todo",
            "todo_id": todo_id,
            "updates": updates,
        });
        
        let response = self.send_request("todo-server", payload).await?;
        Ok(response.message.unwrap_or_else(|| "Todo updated successfully".to_string()))
    }
    
    pub async fn delete_todo(&self, todo_id: &str) -> Result<String> {
        let payload = json!({
            "command": "delete_todo",
            "todo_id": todo_id,
        });
        
        let response = self.send_request("todo-server", payload).await?;
        Ok(response.message.unwrap_or_else(|| "Todo deleted successfully".to_string()))
    }
    
    pub async fn mark_todo_complete(&self, todo_id: &str) -> Result<String> {
        let payload = json!({
            "command": "mark_todo_complete",
            "todo_id": todo_id,
        });
        
        let response = self.send_request("todo-server", payload).await?;
        Ok(response.message.unwrap_or_else(|| "Todo marked complete successfully".to_string()))
    }
}

pub struct McpTodoClientTool {
    client: Arc<McpTodoClient>,
}

impl McpTodoClientTool {
    pub async fn new() -> Result<Self> {
        let client = McpTodoClient::new().await?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait]
impl ToolExecutor for McpTodoClientTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let command = params.get("command").ok_or_else(|| anyhow!("Missing command parameter"))?;
        tracing::debug!("Executing McpTodoClientTool command: {}", command);
        
        match command.as_str() {
            "add" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                let project = params.get("project").ok_or_else(|| anyhow!("Missing project parameter"))?;
                let priority = params.get("priority").map(|s| s.as_str());
                let target_agent = params.get("target_agent").map(|s| s.as_str());
                
                tracing::debug!("Adding todo to MCP Server - Description: {}, Project: {}", description, project);
                let todo_id = self.client.add_todo(description, project, priority, target_agent).await?;
                Ok(format!("Added todo to MCP Server with ID: {}", todo_id))
            },
            "list" => {
                let limit = params.get("limit").map(|s| s.parse::<u32>().unwrap_or(10));
                let filter_str = params.get("filter").map(|s| s.as_str()).unwrap_or("{}");
                let filter = serde_json::from_str(filter_str).map_err(|_| anyhow!("Invalid filter JSON"))?;
                
                tracing::debug!("Listing todos from MCP Server");
                let todos = self.client.query_todos(Some(filter), limit).await?;
                
                if todos.is_empty() {
                    return Ok("No todos found.".to_string());
                }
                
                let mut output = String::from("Todos from MCP Server:\n");
                for todo in todos {
                    output.push_str(&format!("- {} (ID: {}, Status: {})\n", 
                        todo.description, todo.id, todo.status));
                }
                
                Ok(output)
            },
            "get" => {
                let todo_id = params.get("todo_id").ok_or_else(|| anyhow!("Missing todo_id parameter"))?;
                
                tracing::debug!("Getting todo from MCP Server - ID: {}", todo_id);
                let todo = self.client.get_todo(todo_id).await?;
                
                Ok(format!("Todo: {}\nProject: {}\nStatus: {}\nPriority: {}", 
                    todo.description, todo.project, todo.status, todo.priority))
            },
            "update" => {
                let todo_id = params.get("todo_id").ok_or_else(|| anyhow!("Missing todo_id parameter"))?;
                let updates_str = params.get("updates").ok_or_else(|| anyhow!("Missing updates parameter"))?;
                let updates = serde_json::from_str(updates_str).map_err(|_| anyhow!("Invalid updates JSON"))?;
                
                tracing::debug!("Updating todo in MCP Server - ID: {}", todo_id);
                let result = self.client.update_todo(todo_id, updates).await?;
                Ok(result)
            },
            "complete" => {
                let todo_id = params.get("todo_id").ok_or_else(|| anyhow!("Missing todo_id parameter"))?;
                
                tracing::debug!("Marking todo as complete in MCP Server - ID: {}", todo_id);
                let result = self.client.mark_todo_complete(todo_id).await?;
                Ok(result)
            },
            "delete" => {
                let todo_id = params.get("todo_id").ok_or_else(|| anyhow!("Missing todo_id parameter"))?;
                
                tracing::debug!("Deleting todo from MCP Server - ID: {}", todo_id);
                let result = self.client.delete_todo(todo_id).await?;
                Ok(result)
            },
            _ => {
                tracing::error!("Unknown McpTodoClientTool command: {}", command);
                Err(anyhow!("Unknown command: {}", command))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    
    // These tests require a running MCP Todo Server and will actually create/modify todos
    // They are disabled by default and should be run manually when needed
    
    #[tokio::test]
    #[ignore]
    async fn test_add_and_get_todo() -> Result<()> {
        let client = McpTodoClient::new().await?;
        
        // Add a new todo
        let description = format!("Test todo {}", Uuid::new_v4());
        let todo_id = client.add_todo(&description, "test_project", None, None).await?;
        
        // Get the todo we just created
        let todo = client.get_todo(&todo_id).await?;
        
        assert_eq!(todo.id, todo_id);
        assert_eq!(todo.description, description);
        assert_eq!(todo.project, "test_project");
        
        // Clean up - delete the todo
        client.delete_todo(&todo_id).await?;
        
        Ok(())
    }
    
    #[tokio::test]
    #[ignore]
    async fn test_query_todos() -> Result<()> {
        let client = McpTodoClient::new().await?;
        
        // Add a few test todos
        let project = format!("test_project_{}", Uuid::new_v4());
        
        let todo1_id = client.add_todo("Test todo 1", &project, None, None).await?;
        let todo2_id = client.add_todo("Test todo 2", &project, None, None).await?;
        
        // Query todos with filter
        let filter = json!({
            "project": project
        });
        
        let todos = client.query_todos(Some(filter), Some(10)).await?;
        
        assert!(todos.len() >= 2);
        
        // Clean up - delete the todos
        client.delete_todo(&todo1_id).await?;
        client.delete_todo(&todo2_id).await?;
        
        Ok(())
    }
} 
