use std::time::Duration;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde_json::json;
use swarmonomicon::types::TaskPriority;
use serde::Serialize;
use tokio::time;
use std::error::Error;

#[derive(Serialize)]
struct TodoRequest {
    description: String,
    priority: TaskPriority,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("test_mcp_todo_client", "3.134.3.199", 3003);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Wait for connection to be established
    tracing::info!("Connecting to MQTT broker...");

    // Start the eventloop in a separate task
    let client_clone = client.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => {
                    tracing::debug!("Received = {:?}", notification);
                    match notification {
                        Event::Incoming(packet) => tracing::debug!("Incoming Packet = {:?}", packet),
                        Event::Outgoing(packet) => tracing::debug!("Outgoing Packet = {:?}", packet),
                    }
                }
                Err(e) => {
                    tracing::error!("Error from eventloop: {:?}", e);
                    break;
                }
            }
        }
    });

    // Give some time for connection to establish
    time::sleep(Duration::from_secs(2)).await;

    // Create a properly structured request
    let todo_request = TodoRequest {
        description: "brainstorm ideas to expand your own capablities".to_string(),
        priority: TaskPriority::Medium,
    };

    // Serialize to JSON string
    let payload = serde_json::to_string(&todo_request).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    tracing::info!("Sending payload: {}", payload);

    // Publish the todo request with retry logic
    for attempt in 1..=3 {
        match client_clone.publish("mcp/todo/new", QoS::AtLeastOnce, false, payload.clone()).await {
            Ok(_) => {
                tracing::info!("Successfully published test todo task");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    return Err(Box::new(e) as Box<dyn Error>);
                }
                tracing::warn!("Publish attempt {} failed: {}. Retrying...", attempt, e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    // Wait for message to be processed
    time::sleep(Duration::from_secs(5)).await;

    Ok(())
}
