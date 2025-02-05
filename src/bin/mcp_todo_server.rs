use std::time::Duration;
use swarmonomicon::types::{TodoTask, TaskPriority, TaskStatus};
use rumqttc::{MqttOptions, AsyncClient, QoS};
use serde::{Deserialize, Serialize};
use tokio::task;

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: TaskPriority,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mcp_todo_server", "broker.hivemq.com", 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    // Subscribe to "mcp/todo/new" topic
    client.subscribe("mcp/todo/new", QoS::AtMostOnce).await?;

    tracing::info!("MCP Todo Server started. Listening for new tasks...");

    // Handle incoming MCP todo requests
    task::spawn(async move {
        loop {
            let notification = event_loop.poll().await;
            if let Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) = notification {
                let payload = String::from_utf8_lossy(&publish.payload).to_string();
                match serde_json::from_str::<McpTodoRequest>(&payload) {
                    Ok(request) => {
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

                        // Publish new task on "todos/new" topic
                        let payload = serde_json::to_string(&task)?;
                        client.publish("todos/new", QoS::AtLeastOnce, false, payload).await?;

                        tracing::info!("Published new MCP todo: {:?}", task);
                    }
                    Err(e) => tracing::error!("Failed to parse MCP request: {}", e),
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}
