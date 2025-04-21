use swarmonomicon::types::{TodoList, TaskPriority};
use std::collections::HashMap;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Create TodoList instance
    let todo_list = TodoList::new().await?;
    
    // Create metadata
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "feature".to_string());
    metadata.insert("complexity".to_string(), "medium".to_string());
    metadata.insert("estimated_time".to_string(), "3d".to_string());
    metadata.insert("requires".to_string(), "AnythingLLM,API,LocalModels".to_string());

    // Add todo for AnythingLLM integration
    let task = todo_list.create_task_with_enhancement(
        "Implement local LLM support using AnythingLLM for MCP setters".to_string(),
        TaskPriority::High,
        Some("development".to_string()),
        "user".to_string(),
        Some(metadata),
    ).await?;

    println!("✅ Added new Todo task:");
    println!("ID: {}", task.id);
    println!("Description: {}", task.description);
    println!("Priority: {:?}", task.priority);
    println!("Status: {:?}", task.status);
    
    Ok(())
} 