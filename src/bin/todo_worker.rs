use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::{AgentConfig, Message, TodoList, TodoTask, TaskStatus, TaskPriority};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
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

    // Create a shared TodoList
    let todo_list = Arc::new(TodoList::new());

    // Start the MQTT listener in a separate task
    let mut mqtt_options = MqttOptions::new("todo_worker", "broker.hivemq.com", 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    client.subscribe("todos/#", QoS::AtMostOnce).await?;

    let user_agent = Arc::new(user_agent);
    let user_agent_clone = user_agent.clone();
    let todo_list_clone = todo_list.clone();

    task::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(Packet::Publish(publish)) = notification {
                        let description = String::from_utf8_lossy(&publish.payload).to_string();
                        let todo_name = publish.topic.clone();

                        // Create a new TodoTask and add it to the shared TodoList
                        let task = TodoTask {
                            id: uuid::Uuid::new_v4().to_string(),
                            description,
                            priority: TaskPriority::Medium,
                            source_agent: Some("mqtt".to_string()),
                            target_agent: "user".to_string(),
                            status: TaskStatus::Pending,
                            created_at: chrono::Utc::now().timestamp(),
                            completed_at: None,
                        };
                        todo_list_clone.add_task(task).await;
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
        if let Some(task) = todo_list.get_next_task().await {
            tracing::info!("Processing task: {:?}", task);

            // Create a message for the user agent
            let message = Message::new(format!("todo: {}", task.description));
            match user_agent.process_message(message).await {
                Ok(_) => {
                    // Mark the task completed if processing succeeded
                    todo_list.mark_task_completed(&task.id).await;
                    tracing::info!("Task completed");
                }
                Err(e) => {
                    // Mark the task failed if processing failed
                    todo_list.mark_task_failed(&task.id).await;
                    tracing::error!("Task failed: {}", e);
                }
            }
        } else {
            // If no tasks, sleep briefly before checking again
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
