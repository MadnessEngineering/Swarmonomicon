use anyhow::Result;
use swarmonomicon::tools::{McpTodoClientTool, ToolExecutor};
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    env_logger::init();
    
    // Set environment variables for MQTT connection (modify as needed)
    env::set_var("MQTT_HOST", "localhost");
    env::set_var("MQTT_PORT", "1883");
    
    println!("Initializing MCP Todo Client Tool...");
    let tool = McpTodoClientTool::new().await?;
    
    // Example 1: Add a todo
    println!("\n=== ADDING A TODO ===");
    let mut params = HashMap::new();
    params.insert("command".to_string(), "add".to_string());
    params.insert("description".to_string(), "Example todo from Swarmonomicon MCP Todo Client".to_string());
    params.insert("project".to_string(), "examples".to_string());
    params.insert("priority".to_string(), "medium".to_string());
    
    match tool.execute(params).await {
        Ok(result) => println!("Result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    // Example 2: List todos
    println!("\n=== LISTING TODOS ===");
    let mut params = HashMap::new();
    params.insert("command".to_string(), "list".to_string());
    params.insert("filter".to_string(), r#"{"project": "examples"}"#.to_string());
    params.insert("limit".to_string(), "5".to_string());
    
    match tool.execute(params).await {
        Ok(result) => println!("Result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    // Example 3: Get a specific todo
    // First, add a new todo and capture its ID
    println!("\n=== CREATING A TODO FOR GET EXAMPLE ===");
    let mut params = HashMap::new();
    params.insert("command".to_string(), "add".to_string());
    params.insert("description".to_string(), "Todo for GET example".to_string());
    params.insert("project".to_string(), "examples".to_string());
    
    let todo_id = match tool.execute(params).await {
        Ok(result) => {
            println!("Added todo: {}", result);
            // Extract ID from result (assuming format "Added todo to MCP Server with ID: <id>")
            result.split_whitespace().last().unwrap_or("").trim_end_matches('.').to_string()
        },
        Err(e) => {
            println!("Error: {}", e);
            "".to_string()
        },
    };
    
    if !todo_id.is_empty() {
        println!("\n=== GETTING A SPECIFIC TODO ===");
        let mut params = HashMap::new();
        params.insert("command".to_string(), "get".to_string());
        params.insert("todo_id".to_string(), todo_id.clone());
        
        match tool.execute(params).await {
            Ok(result) => println!("Result: {}", result),
            Err(e) => println!("Error: {}", e),
        }
        
        // Example 4: Update a todo
        println!("\n=== UPDATING A TODO ===");
        let mut params = HashMap::new();
        params.insert("command".to_string(), "update".to_string());
        params.insert("todo_id".to_string(), todo_id.clone());
        params.insert("updates".to_string(), r#"{"priority": "high", "status": "in-progress"}"#.to_string());
        
        match tool.execute(params).await {
            Ok(result) => println!("Result: {}", result),
            Err(e) => println!("Error: {}", e),
        }
        
        // Example 5: Complete a todo
        println!("\n=== COMPLETING A TODO ===");
        let mut params = HashMap::new();
        params.insert("command".to_string(), "complete".to_string());
        params.insert("todo_id".to_string(), todo_id.clone());
        
        match tool.execute(params).await {
            Ok(result) => println!("Result: {}", result),
            Err(e) => println!("Error: {}", e),
        }
    }
    
    println!("\nAll examples completed!");
    Ok(())
} 
