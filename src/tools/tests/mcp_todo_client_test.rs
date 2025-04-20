use crate::tools::{McpTodoClientTool, ToolExecutor};
use std::collections::HashMap;
use serde_json::json;

// Note: These tests require a running MCP Todo Server to communicate with.
// They are marked as `#[ignore]` so they don't run during normal test suite execution.
// To run these tests, use: cargo test -- --ignored

#[tokio::test]
#[ignore]
async fn test_mcp_todo_client_add_todo() {
    // This test adds a todo item to the MCP Todo Server via MQTT
    println!("Initializing MCP Todo Client Tool...");
    let tool = match McpTodoClientTool::new().await {
        Ok(t) => t,
        Err(e) => {
            println!("Failed to initialize McpTodoClientTool: {}", e);
            panic!("Test failed to initialize tool: {}", e);
        }
    };
    
    println!("Creating test parameters...");
    let mut params = HashMap::new();
    params.insert("command".to_string(), "add".to_string());
    params.insert("description".to_string(), "Test todo from MCP Todo Client Tool".to_string());
    params.insert("project".to_string(), "swarmonomicon-test".to_string());
    params.insert("priority".to_string(), "medium".to_string());
    
    println!("Executing add command...");
    let result = tool.execute(params).await;
    
    match result {
        Ok(output) => {
            println!("Add todo result: {}", output);
            assert!(output.contains("Added todo to MCP Server with ID"), 
                    "Expected output to contain ID confirmation, got: {}", output);
        },
        Err(e) => {
            println!("Failed to add todo: {}", e);
            panic!("Test failed: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_mcp_todo_client_list_todos() {
    // This test lists todos from the MCP Todo Server via MQTT
    println!("Initializing MCP Todo Client Tool...");
    let tool = match McpTodoClientTool::new().await {
        Ok(t) => t,
        Err(e) => {
            println!("Failed to initialize McpTodoClientTool: {}", e);
            panic!("Test failed to initialize tool: {}", e);
        }
    };
    
    println!("Creating test parameters...");
    let mut params = HashMap::new();
    params.insert("command".to_string(), "list".to_string());
    params.insert("filter".to_string(), r#"{"project": "swarmonomicon-test"}"#.to_string());
    params.insert("limit".to_string(), "5".to_string());
    
    println!("Executing list command...");
    let result = tool.execute(params).await;
    
    match result {
        Ok(output) => {
            println!("List todos result: {}", output);
            // We don't assert specific content since the database might have different todos
            // Just check that it doesn't error and has the expected format
            assert!(output.contains("Todos from MCP Server") || output.contains("No todos found"),
                    "Expected output to be a list of todos or empty, got: {}", output);
        },
        Err(e) => {
            println!("Failed to list todos: {}", e);
            panic!("Test failed: {}", e);
        }
    }
}

// Add this test to mod.rs
// pub mod mcp_todo_client_test; 
