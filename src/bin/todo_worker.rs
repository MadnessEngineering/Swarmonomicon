use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::{AgentConfig, Message, TodoList, TodoTask, TaskStatus, TaskPriority, Tool};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use tokio::task;
use std::collections::HashMap;
use std::sync::Arc;
use swarmonomicon::tools::ToolRegistry;
use anyhow::{Result, anyhow};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get the global registry
    let registry = agents::GLOBAL_REGISTRY.clone();
    let mut registry = registry.write().await;

    // Create a tool registry
    let tool_registry = ToolRegistry::create_default_tools().await?;
    let tool_registry = Arc::new(tool_registry);

    // Create a user agent for handling todos
    let user_agent = UserAgent::new(AgentConfig {
        name: "user".to_string(),
        public_description: "User agent for handling todos".to_string(),
        instructions: "Handles todo operations".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    });

    // Create a shared TodoList
    let todo_list = Arc::new(TodoList::new().await.map_err(|e| anyhow!("Failed to create TodoList: {}", e))?);

    // Start the MQTT listener in a separate task
    let mut mqtt_options = MqttOptions::new("todo_worker", "3.134.3.199", 3003);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    client.subscribe("todos/#", QoS::AtMostOnce).await.map_err(|e| anyhow!("Failed to subscribe to MQTT topic: {}", e))?;

    let user_agent = Arc::new(user_agent);
    let user_agent_clone = user_agent.clone();
    let todo_list_clone = todo_list.clone();
    let tool_registry_clone = tool_registry.clone();

    task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(Packet::Publish(publish)) = notification {
                        let description = String::from_utf8_lossy(&publish.payload).to_string();
                        let todo_name = publish.topic.clone();

                        // Create a new TodoTask with optional AI enhancement
                        match todo_list_clone.create_task_with_enhancement(
                            description,
                            TaskPriority::Medium,
                            Some("mqtt".to_string()),
                            "user".to_string(),
                            None, // Project field
                            None, // No AI enhancement at worker level
                        ).await {
                            Ok(task) => {
                                tracing::info!("Successfully created task: {:?}", task);
                            }
                            Err(e) => {
                                tracing::error!("Failed to create task: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Connection error: {}", e);
                    break;
                }
            }
        }
    });

    tracing::info!("Todo worker started. Checking for tasks...");

    // Main worker loop to process tasks from the TodoList
    loop {
        // Get the next task from the TodoList, if any
        if let Some(task) = todo_list.get_next_task().await.map_err(|e| anyhow!("Failed to get next task: {}", e))? {
            tracing::info!("Processing task: {:?}", task);

            // Parse the task description to determine if it needs Goose
            let needs_goose = task.description.to_lowercase().contains("edit") 
                || task.description.to_lowercase().contains("run") 
                || task.description.to_lowercase().contains("execute");

            if needs_goose {
                // Use Goose tool to handle the task
                let mut params = HashMap::new();
                
                if task.description.to_lowercase().contains("edit") {
                    params.insert("action".to_string(), "edit".to_string());
                    // Extract file path and instructions from description
                    // This is a simple example - you'd want more sophisticated parsing
                    let parts: Vec<&str> = task.description.split(" ").collect();
                    if parts.len() >= 4 {
                        params.insert("file_path".to_string(), parts[1].to_string());
                        params.insert("instructions".to_string(), parts[3..].join(" "));
                    }
                } else {
                    params.insert("action".to_string(), "exec".to_string());
                    // Extract command from description
                    let command = task.description.replace("run", "").replace("execute", "").trim().to_string();
                    params.insert("command".to_string(), command);
                }

                let goose_tool = Tool {
                    name: "goose".to_string(),
                    description: "Goose CLI tool".to_string(),
                    parameters: params.clone(),
                };

                match tool_registry.execute(&goose_tool, params).await {
                    Ok(result) => {
                        tracing::info!("Goose task completed: {}", result);
                        todo_list.mark_task_completed(&task.id).await.map_err(|e| anyhow!("Failed to mark task completed: {}", e))?;
                    }
                    Err(e) => {
                        tracing::error!("Goose task failed: {}", e);
                        todo_list.mark_task_failed(&task.id).await.map_err(|e| anyhow!("Failed to mark task failed: {}", e))?;
                    }
                }
            } else {
                // Create a message for the user agent
                let message = Message::new(format!("todo: {}", task.description));
                match user_agent.process_message(message).await {
                    Ok(_) => {
                        // Mark the task completed if processing succeeded
                        todo_list.mark_task_completed(&task.id).await.map_err(|e| anyhow!("Failed to mark task completed: {}", e))?;
                        tracing::info!("Task completed");
                    }
                    Err(e) => {
                        // Mark the task failed if processing failed
                        todo_list.mark_task_failed(&task.id).await.map_err(|e| anyhow!("Failed to mark task failed: {}", e))?;
                        tracing::error!("Task failed: {}", e);
                    }
                }
            }
        } else {
            // If no tasks, sleep briefly before checking again
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
