use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::{AgentConfig, Message, TodoList, TodoTask, TaskStatus, TaskPriority, Tool};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use tokio::{task, sync::Semaphore, time};
use std::collections::HashMap;
use std::sync::Arc;
use swarmonomicon::tools::ToolRegistry;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

// Maximum number of concurrent tasks to process
const MAX_CONCURRENT_TASKS: usize = 5;
// Maximum retries for task processing
const MAX_RETRIES: usize = 3;
// Backoff time for retries (in seconds)
const RETRY_BACKOFF: u64 = 5;
// Poll interval for checking tasks (in seconds)
const POLL_INTERVAL: u64 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct TaskPayload {
    description: String,
    priority: Option<TaskPriority>,
    context: Option<String>,
    target_agent: Option<String>,
    metadata: Option<HashMap<String, String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with more verbose output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("Starting todo_worker...");

    // Get the global registry
    let registry = agents::GLOBAL_REGISTRY.clone();
    let mut registry = registry.write().await;

    // Create a tool registry
    tracing::info!("Initializing tool registry...");
    let tool_registry = ToolRegistry::create_default_tools().await?;
    let tool_registry = Arc::new(tool_registry);

    // Create a user agent for handling todos
    tracing::info!("Creating user agent...");
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
    tracing::info!("Initializing TodoList...");
    let todo_list = Arc::new(TodoList::new().await.map_err(|e| anyhow!("Failed to create TodoList: {}", e))?);

    // Rate limiter for task processing
    let task_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));

    // Start the MQTT listener in a separate task
    tracing::info!("Connecting to MQTT broker...");
    let mut mqtt_options = MqttOptions::new(
        "todo_worker", 
        &std::env::var("AWSIP").unwrap_or_else(|_| "localhost".to_string()), 
        std::env::var("MQTTPORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(3003)
    );
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    // Subscribe to relevant topics with retry logic
    tracing::info!("Subscribing to MQTT topics...");
    for attempt in 1..=3 {
        match client.subscribe("todos/#", QoS::AtLeastOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to todos/#");
                match client.subscribe("mcp/task/#", QoS::AtLeastOnce).await {
                    Ok(_) => {
                        tracing::info!("Successfully subscribed to mcp/task/#");
                        break;
                    },
                    Err(e) => {
                        tracing::warn!("Failed to subscribe to mcp/task/#: {}", e);
                        if attempt == 3 {
                            return Err(anyhow!("Failed to subscribe to mcp/task/# after 3 attempts"));
                        }
                    }
                }
            },
            Err(e) => {
                tracing::warn!("Failed to subscribe to todos/#: {}", e);
                if attempt == 3 {
                    return Err(anyhow!("Failed to subscribe to todos/# after 3 attempts"));
                }
            }
        }
        time::sleep(Duration::from_secs(1)).await;
    }

    let user_agent = Arc::new(user_agent);
    let todo_list_clone = todo_list.clone();
    let client_clone = client.clone();

    // MQTT listener task
    tracing::info!("Starting MQTT listener...");
    task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(Packet::Publish(publish)) = notification {
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        let topic = publish.topic.clone();
                        tracing::debug!("Received message on topic {}: {}", topic, payload);

                        // Clone necessary data for the task processing
                        let todo_list = todo_list_clone.clone();
                        
                        // Process payload based on topic
                        task::spawn(async move {
                            if topic.starts_with("todos/") {
                                // Process simple todo message
                                let description = payload;
                                let priority = TaskPriority::Medium;
                                let context = Some("mqtt".to_string());
                                let target_agent = topic.split('/').nth(1).unwrap_or("user").to_string();

                                match todo_list.create_task_with_enhancement(
                                    description,
                                    priority,
                                    context,
                                    target_agent,
                                    None, // No AI enhancement at worker level
                                ).await {
                                    Ok(task) => {
                                        tracing::info!("Created task from todos/ topic: {}", task.id);
                                    },
                                    Err(e) => {
                                        tracing::error!("Failed to create task from todos/ topic: {}", e);
                                    }
                                }
                            } else if topic.starts_with("mcp/task/") {
                                // Process structured task payload
                                match serde_json::from_str::<TaskPayload>(&payload) {
                                    Ok(task_payload) => {
                                        let priority = task_payload.priority.unwrap_or(TaskPriority::Medium);
                                        let context = task_payload.context.or(Some("mcp".to_string()));
                                        let target_agent = task_payload.target_agent.unwrap_or_else(|| {
                                            topic.split('/').nth(2).unwrap_or("user").to_string()
                                        });
                                        
                                        match todo_list.create_task_with_enhancement(
                                            task_payload.description,
                                            priority,
                                            context,
                                            target_agent,
                                            task_payload.metadata,
                                        ).await {
                                            Ok(task) => {
                                                tracing::info!("Created task from mcp/task topic: {}", task.id);
                                            },
                                            Err(e) => {
                                                tracing::error!("Failed to create task from mcp/task topic: {}", e);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        // If JSON parsing fails, try to use it as simple description
                                        tracing::warn!("Failed to parse task JSON: {}. Using as plain text.", e);
                                        let description = payload;
                                        let target_agent = topic.split('/').nth(2).unwrap_or("user").to_string();
                                        
                                        match todo_list.create_task_with_enhancement(
                                            description,
                                            TaskPriority::Medium,
                                            Some("mcp".to_string()),
                                            target_agent,
                                            None,
                                        ).await {
                                            Ok(task) => {
                                                tracing::info!("Created task from mcp/task topic as plain text: {}", task.id);
                                            },
                                            Err(e) => {
                                                tracing::error!("Failed to create task from mcp/task topic as plain text: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                },
                Err(e) => {
                    tracing::error!("MQTT connection error: {}", e);
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    tracing::info!("Todo worker started. Task processor initialized.");

    // Task processing loop
    let user_agent_clone = user_agent.clone();
    let tool_registry_clone = tool_registry.clone();
    let todo_list_clone = todo_list.clone();
    let client_feedback = client_clone.clone();
    
    loop {
        // Try to acquire a permit for task processing
        let permit = match task_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                tracing::error!("Failed to acquire task semaphore: {}", e);
                time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Get the next task from the TodoList, if any
        match todo_list_clone.get_next_task().await {
            Ok(Some(task)) => {
                tracing::info!("Processing task: {} - {}", task.id, task.description);
                
                // Clone necessary data for the task processing
                let user_agent = user_agent_clone.clone();
                let tool_registry = tool_registry_clone.clone();
                let todo_list = todo_list_clone.clone();
                let client = client_feedback.clone();
                let task_id = task.id.clone();
                let task_description = task.description.clone();
                let target_agent = task.target_agent.clone();

                // Spawn a task to process this task independently
                task::spawn(async move {
                    // The permit will be dropped when this task completes
                    let _permit = permit;
                    
                    // Send started notification
                    let status_payload = serde_json::json!({
                        "id": task_id,
                        "status": "started",
                        "description": task_description,
                    }).to_string();
                    
                    let _ = client.publish(
                        format!("task/status/{}", target_agent), 
                        QoS::AtLeastOnce, 
                        false, 
                        status_payload
                    ).await;

                    // Process the task with retry logic
                    let mut success = false;
                    let mut retry_count = 0;
                    let mut last_error = None;

                    while retry_count < MAX_RETRIES && !success {
                        if retry_count > 0 {
                            tracing::info!("Retrying task {} (attempt {}/{})", task_id, retry_count + 1, MAX_RETRIES);
                            time::sleep(Duration::from_secs(RETRY_BACKOFF)).await;
                        }

                        // Parse the task to determine how to process it
                        let needs_goose = task_description.to_lowercase().contains("edit") 
                            || task_description.to_lowercase().contains("run") 
                            || task_description.to_lowercase().contains("execute");

                        if needs_goose {
                            // Use Goose tool to handle the task
                            let mut params = HashMap::new();
                            
                            if task_description.to_lowercase().contains("edit") {
                                params.insert("action".to_string(), "edit".to_string());
                                // Extract file path and instructions from description
                                // This is a simple example - you'd want more sophisticated parsing
                                let parts: Vec<&str> = task_description.split(" ").collect();
                                if parts.len() >= 4 {
                                    params.insert("file_path".to_string(), parts[1].to_string());
                                    params.insert("instructions".to_string(), parts[3..].join(" "));
                                }
                            } else {
                                params.insert("action".to_string(), "exec".to_string());
                                // Extract command from description
                                let command = task_description.replace("run", "").replace("execute", "").trim().to_string();
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
                                    success = true;
                                }
                                Err(e) => {
                                    tracing::error!("Goose task failed: {}", e);
                                    last_error = Some(e.to_string());
                                    retry_count += 1;
                                }
                            }
                        } else {
                            // Create a message for the user agent
                            let message = Message::new(format!("todo: {}", task_description));
                            match user_agent.process_message(message).await {
                                Ok(_) => {
                                    tracing::info!("Task processed through user agent");
                                    success = true;
                                }
                                Err(e) => {
                                    tracing::error!("User agent task failed: {}", e);
                                    last_error = Some(e.to_string());
                                    retry_count += 1;
                                }
                            }
                        }
                    }

                    // Update task status based on processing result
                    let result = if success {
                        todo_list.mark_task_completed(&task_id).await.map(|_| "completed".to_string())
                    } else {
                        let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
                        todo_list.mark_task_failed(&task_id).await.map(|_| format!("failed: {}", error_msg))
                    };

                    // Send status update via MQTT
                    match result {
                        Ok(status) => {
                            let status_payload = serde_json::json!({
                                "id": task_id,
                                "status": status,
                                "description": task_description,
                            }).to_string();
                            
                            let _ = client.publish(
                                format!("task/status/{}", target_agent), 
                                QoS::AtLeastOnce, 
                                false, 
                                status_payload
                            ).await;
                            
                            tracing::info!("Task {} status updated to {}", task_id, status);
                        },
                        Err(e) => {
                            tracing::error!("Failed to update task status: {}", e);
                        }
                    }
                });
            },
            Ok(None) => {
                // No tasks available, release permit and sleep before checking again
                drop(permit);
                time::sleep(Duration::from_secs(POLL_INTERVAL)).await;
            },
            Err(e) => {
                tracing::error!("Failed to get next task: {}", e);
                drop(permit);
                time::sleep(Duration::from_secs(POLL_INTERVAL)).await;
            }
        }
    }
}
