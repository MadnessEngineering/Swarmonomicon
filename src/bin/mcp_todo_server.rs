use std::time::Duration;
use swarmonomicon::types::{TodoTask, TaskPriority, TaskStatus};
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde::{Deserialize, Serialize};
use tokio::{task, time};

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: TaskPriority,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with more verbose output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mcp_todo_server", "3.134.3.199", 3003);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    mqtt_options.set_connection_timeout_ms(10000);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    // Wait for connection and subscribe
    tracing::info!("Connecting to MQTT broker...");

    // Subscribe to "mcp/todo/new" topic with retry logic
    for attempt in 1..=3 {
        match client.subscribe("mcp/todo/new", QoS::AtLeastOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to mcp/todo/new");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    return Err(Box::new(e));
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
    task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    tracing::debug!("Received notification: {:?}", notification);
                    if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        tracing::info!("Received payload: {}", payload);

                        match serde_json::from_str::<McpTodoRequest>(&payload) {
                            Ok(request) => {
                                tracing::info!("Parsed request: {:?}", request);
                                let task = TodoTask {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    description: request.description,
                                    priority: request.priority,
                                    source_agent: Some("mcp_server".to_string()),
                                    target_agent: "user".to_string(),
                                    status: TaskStatus::Pending,
                                    created_at: chrono::Utc::now().timestamp(),
                                    completed_at: None,
                                };

                                // Handle serialization and publishing errors
                                match serde_json::to_string(&task) {
                                    Ok(payload) => {
                                        match client_clone.publish("todos/new", QoS::AtLeastOnce, false, payload).await {
                                            Ok(_) => tracing::info!("Published new MCP todo: {:?}", task),
                                            Err(e) => tracing::error!("Failed to publish task: {}", e),
                                        }
                                    }
                                    Err(e) => tracing::error!("Failed to serialize task: {}", e),
                                }
                            }
                            Err(e) => tracing::error!("Failed to parse MCP request: {}\nPayload was: {}", e, payload),
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

    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to listen for ctrl-c: {}", e);
    }
    Ok(())
}
