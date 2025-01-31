use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::{AgentConfig, Message};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, Client, QoS, Event, Packet, EventLoop};
use tokio::task;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get the global registry
    let registry = agents::GLOBAL_REGISTRY.clone();
    let mut registry = registry.write().await;

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

    // Start the MQTT listener in a separate task
    let mut mqtt_options = MqttOptions::new("todo_worker", "broker.hivemq.com", 1883);
    mqtt_options.set_keep_alive(5); // 5 seconds as u16
    let (mut client, mut event_loop) = Client::new(mqtt_options, 10);
    
    client.subscribe("todos/#", QoS::AtMostOnce)?;

    let user_agent = Arc::new(user_agent);
    let user_agent_clone = user_agent.clone();

    task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(Packet::Publish(publish)) = notification {
                        let description = String::from_utf8_lossy(&publish.payload).to_string();
                        let todo_name = publish.topic.clone();
                        
                        // Create a message for the user agent
                        let message = Message::new(format!("add todo: {} - {}", todo_name, description));
                        if let Err(e) = user_agent_clone.process_message(message).await {
                            tracing::error!("Failed to process todo: {}", e);
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

    tracing::info!("Todo worker started. Checking for tasks every 2 minutes...");

    // Keep the main task running
    loop {
        tokio::time::sleep(Duration::from_secs(120)).await;
        tracing::info!("Todo worker still running...");
    }
}
