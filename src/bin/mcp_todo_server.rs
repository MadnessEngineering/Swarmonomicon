use std::time::Duration;
use std::collections::HashMap;
use swarmonomicon::types::{TodoTask, TaskPriority, TaskStatus};
use swarmonomicon::tools::todo::TodoTool;
use swarmonomicon::tools::ToolExecutor;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde::{Deserialize, Serialize};
use tokio::{task, time, sync::Semaphore};
use std::error::Error as StdError;
use std::process::Command;
use anyhow::{Result, anyhow};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: Option<TaskPriority>,
}

// Maximum number of concurrent task processing
const MAX_CONCURRENT_TASKS: usize = 5;
// Maximum number of concurrent AI enhancements
const MAX_CONCURRENT_AI: usize = 2;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with more verbose output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Initialize TodoTool
    let todo_tool = Arc::new(TodoTool::new().await.map_err(|e| anyhow!("Failed to initialize TodoTool: {}", e))?);

    // Create semaphores for rate limiting
    let task_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
    let ai_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_AI));

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mcp_todo_server", &std::env::var("AWSIP").expect("AWSIP environment variable not set"), 3003);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    tracing::info!("Connecting to MQTT broker...");

    // Subscribe to "mcp/*" topic with retry logic
    for attempt in 1..=3 {
        match client.subscribe("mcp/#", QoS::AtLeastOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to mcp/#");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    return Err(anyhow!("Failed to subscribe after 3 attempts: {}", e));
                }
                tracing::warn!("Subscribe attempt {} failed: {}. Retrying...", attempt, e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    tracing::info!("MCP Todo Server started. Listening for new tasks...");

    // Create a client clone for the task
    let client_clone = client.clone();

    // Handle incoming MCP todo requests
    let _handler = task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        tracing::info!("Received payload: {}", payload);

                        // Clone necessary Arc's for the task
                        let todo_tool = todo_tool.clone();
                        let task_semaphore = task_semaphore.clone();
                        let ai_semaphore = ai_semaphore.clone();
                        let topic = publish.topic.clone();

                        // Spawn a new task to handle this request
                        tokio::spawn(async move {
                            // Acquire task processing permit
                            let _task_permit = match task_semaphore.acquire().await {
                                Ok(permit) => permit,
                                Err(e) => {
                                    tracing::error!("Failed to acquire task permit: {}", e);
                                    return;
                                }
                            };

                            // Try to parse as McpTodoRequest, if fails treat as plain text
                            let description = match serde_json::from_str::<McpTodoRequest>(&payload) {
                                Ok(request) => request.description,
                                Err(_) => payload,
                            };

                            let target_agent = topic.split('/').nth(1).unwrap_or("user");

                            // Add todo using TodoTool
                            let mut params = HashMap::new();
                            params.insert("command".to_string(), "add".to_string());
                            params.insert("description".to_string(), description.clone());
                            params.insert("context".to_string(), "mcp_server".to_string());
                            params.insert("target_agent".to_string(), target_agent.to_string());

                            // Acquire AI enhancement permit before processing
                            let _ai_permit = match ai_semaphore.acquire().await {
                                Ok(permit) => permit,
                                Err(e) => {
                                    tracing::error!("Failed to acquire AI permit: {}", e);
                                    return;
                                }
                            };

                            match todo_tool.execute(params).await {
                                Ok(_) => tracing::info!("Successfully added todo: {}", description),
                                Err(e) => tracing::error!("Failed to add todo: {}", e),
                            }

                            // AI permit is automatically released here
                            // Task permit is automatically released here
                        });
                    }
                }
                Err(e) => {
                    tracing::error!("Error from eventloop: {:?}", e);
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for ctrl-c
    tokio::signal::ctrl_c().await.map_err(|e| anyhow!("Failed to wait for ctrl-c: {}", e))?;
    Ok(())
}
