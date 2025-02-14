use std::time::Duration;
use std::collections::HashMap;
use swarmonomicon::types::{TodoTask, TaskPriority, TaskStatus};
use swarmonomicon::tools::todo::TodoTool;
use swarmonomicon::tools::ToolExecutor;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde::{Deserialize, Serialize};
use tokio::{task, time};
use std::error::Error as StdError;
use std::process::Command;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: Option<TaskPriority>,
}

async fn enhance_todo_with_ai(description: &str) -> Result<(String, TaskPriority), Box<dyn StdError>> {
    // Use goose CLI to enhance the todo description and guess priority
    let output = Command::new("goose")
        .arg("chat")
        .arg("-m")
        .arg(format!(
            "Given this todo task: '{}', please analyze it and return a JSON object with two fields: \
             1. An enhanced description with more details if possible \
             2. A suggested priority level (low, medium, or high) based on the task's urgency and importance. \
             Format: {{\"description\": \"enhanced text\", \"priority\": \"priority_level\"}}",
            description
        ))
        .output()?;

    let ai_response = String::from_utf8(output.stdout)?;
    let enhanced: serde_json::Value = serde_json::from_str(&ai_response)?;

    let priority = match enhanced["priority"].as_str().unwrap_or("medium") {
        "high" => TaskPriority::High,
        "low" => TaskPriority::Low,
        _ => TaskPriority::Medium,
    };

    Ok((enhanced["description"].as_str().unwrap_or(description).to_string(), priority))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with more verbose output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Initialize TodoTool
    let todo_tool = TodoTool::new().await.map_err(|e| anyhow!("Failed to initialize TodoTool: {}", e))?;

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mcp_todo_server", &std::env::var("AWSIP").expect("AWSIP environment variable not set"), 3003);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    tracing::info!("Connecting to MQTT broker...");

    // Subscribe to "mcp/*" topic with retry logic
    for attempt in 1..=3 {
        match client.subscribe("mcp/*", QoS::AtLeastOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to mcp/*");
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
    let todo_tool_clone = todo_tool.clone();

    // Handle incoming MCP todo requests
    let _handler = task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        tracing::info!("Received payload: {}", payload);

                        // Try to parse as McpTodoRequest, if fails treat as plain text
                        let description = match serde_json::from_str::<McpTodoRequest>(&payload) {
                            Ok(request) => request.description,
                            Err(_) => payload,
                        };

                        let topic = publish.topic;
                        let target_agent = topic.split('/').nth(1).unwrap_or("user");

                        // Add todo using TodoTool
                        let mut params = HashMap::new();
                        params.insert("command".to_string(), "add".to_string());
                        params.insert("description".to_string(), description.clone());
                        params.insert("context".to_string(), "mcp_server".to_string());
                        params.insert("target_agent".to_string(), target_agent.to_string());

                        match todo_tool_clone.execute(params).await {
                            Ok(_) => tracing::info!("Successfully added todo: {}", description),
                            Err(e) => tracing::error!("Failed to add todo: {}", e),
                        }
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
