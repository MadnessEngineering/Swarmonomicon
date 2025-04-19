use std::time::Duration;
use swarmonomicon::agents::{self, AgentRegistry, AgentWrapper};
use swarmonomicon::types::{AgentConfig, Message, TodoList, TodoTask, TaskStatus, TaskPriority};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet, EventLoop};
use tokio::task;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use swarmonomicon::tools::ToolRegistry;
use anyhow::{Result, anyhow};
use std::env;
use std::time::Instant;
use serde_json;
use chrono;
use tracing;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    // Initialize the tracing subscriber
    tracing_subscriber::fmt::init();
    tracing::info!("Starting todo worker");

    // Parse MQTT configuration
    let mqtt_host = env::var("MQTT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let mqtt_port: u16 = env::var("MQTT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1883);
    let mqtt_username = env::var("MQTT_USERNAME").ok();
    let mqtt_password = env::var("MQTT_PASSWORD").ok();
    let mqtt_client_id = env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "todo_worker".to_string());

    tracing::info!("Connecting to MQTT broker at {}:{}", mqtt_host, mqtt_port);

    // Set up MQTT client
    let mut mqtt_options = MqttOptions::new(mqtt_client_id, mqtt_host, mqtt_port);
    mqtt_options.set_keep_alive(Duration::from_secs(20));
    mqtt_options.set_clean_session(true);

    if let (Some(username), Some(password)) = (mqtt_username, mqtt_password) {
        mqtt_options.set_credentials(username, password);
    }

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 100);

    // Subscribe to the topics
    client.subscribe("agent/+/todo/process", QoS::AtLeastOnce).await?;
    tracing::info!("Subscribed to topic: agent/+/todo/process");

    // Create agent registry
    let agent_registry = Arc::new(RwLock::new(AgentRegistry::new()));
    
    // Create default agents
    let config_agents = agents::default_agents();
    for config in config_agents {
        let agent_config = config.clone();
        if let Ok(agent) = agents::create_agent(agent_config).await {
            let mut registry = agent_registry.write().await;
            registry.register(config.name, agent).await?;
        }
    }
    
    // Processing metrics
    let mut tasks_processed = 0;
    let mut tasks_succeeded = 0;
    let mut tasks_failed = 0;
    let start_time = Instant::now();

    // Main loop
    loop {
        match eventloop.poll().await {
            Ok(event) => {
                if let Event::Incoming(Packet::Publish(publish)) = event {
                    let topic = publish.topic.clone();
                    let payload = match std::str::from_utf8(&publish.payload) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to parse payload as UTF-8: {}", e);
                            continue;
                        }
                    };
                    
                    tracing::debug!("Received message on topic {}: {}", topic, payload);
                    
                    // Extract the agent name from the topic
                    if let Some(agent_name) = topic.split('/').nth(1) {
                        tracing::info!("Processing todo for agent: {}", agent_name);
                        
                        match process_todo_for_agent(&agent_registry, agent_name, payload, &client).await {
                            Ok(_) => {
                                tasks_processed += 1;
                                tasks_succeeded += 1;
                                
                                // Report metrics every 10 tasks
                                if tasks_processed % 10 == 0 {
                                    report_metrics(
                                        tasks_processed, 
                                        tasks_succeeded, 
                                        tasks_failed, 
                                        start_time.elapsed(),
                                        &client
                                    ).await?;
                                }
                            },
                            Err(e) => {
                                tasks_processed += 1;
                                tasks_failed += 1;
                                tracing::error!("Failed to process todo for agent {}: {}", agent_name, e);
                                
                                // Publish error message to MQTT
                                let error_topic = format!("agent/{}/todo/error", agent_name);
                                let error_payload = serde_json::json!({
                                    "error": e.to_string(),
                                    "payload": payload
                                }).to_string();
                                
                                if let Err(e) = client.publish(error_topic, QoS::AtLeastOnce, false, error_payload).await {
                                    tracing::error!("Failed to publish error message: {}", e);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                tracing::error!("Error from MQTT eventloop: {}", e);
                // Short delay to avoid tight loop on errors
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                // Optionally check for agent tasks
                if let Err(e) = check_agent_tasks(&agent_registry, &client).await {
                    tracing::error!("Error checking agent tasks: {}", e);
                }
            }
        }
    }
}

async fn process_todo_for_agent(
    agent_registry: &Arc<RwLock<AgentRegistry>>,
    agent_name: &str,
    payload: &str,
    mqtt_client: &AsyncClient,
) -> Result<()> {
    // Parse task from payload
    let task: TodoTask = match serde_json::from_str(payload) {
        Ok(task) => task,
        Err(e) => {
            tracing::error!("Failed to parse task payload: {}", e);
            return Err(anyhow::anyhow!("Invalid task payload: {}", e));
        }
    };
    
    // Get priority level as a string for logging
    let priority_str = match &task.priority {
        TaskPriority::Low => "low",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
        TaskPriority::Critical => "critical",
    };
    tracing::info!("Processing task {} with priority {}", task.id, priority_str);
    
    // Get agent to process the task
    let registry = agent_registry.read().await;
    let agent = match registry.get(agent_name) {
        Some(agent) => agent,
        None => {
            tracing::error!("Agent not found: {}", agent_name);
            return Err(anyhow::anyhow!("Agent not found: {}", agent_name));
        }
    };
    
    // Process the task
    match agent.process_task(task.clone()).await {
        Ok(response) => {
            // Publish response
            let response_topic = format!("agent/{}/todo/response", agent_name);
            let response_payload = serde_json::json!({
                "task_id": task.id,
                "message": response.content,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string();
            
            mqtt_client.publish(response_topic, QoS::AtLeastOnce, false, response_payload).await?;
            
            // Mark task as completed
            if let Some(todo_list) = agent.get_todo_list() {
                if let Err(e) = todo_list.mark_task_completed(&task.id).await {
                    tracing::error!("Failed to mark task as completed: {}", e);
                }
            }
            
            Ok(())
        },
        Err(e) => {
            tracing::error!("Failed to process task {}: {}", task.id, e);
            
            // Mark task as failed
            if let Some(todo_list) = agent.get_todo_list() {
                if let Err(mark_err) = todo_list.mark_task_failed(&task.id).await {
                    tracing::error!("Failed to mark task as failed: {}", mark_err);
                }
            }
            
            Err(anyhow::anyhow!("Failed to process task: {}", e))
        }
    }
}

async fn check_agent_tasks(agent_registry: &Arc<RwLock<AgentRegistry>>, mqtt_client: &AsyncClient) -> Result<()> {
    let registry = agent_registry.read().await;
    let agent_names: Vec<String> = registry.iter().map(|(name, _)| name.clone()).collect();
    
    for agent_name in agent_names {
        if let Some(agent) = registry.get(&agent_name) {
            let check_interval = agent.get_check_interval();
            let todo_list = agent.get_todo_list();
            
            // Check if there are any tasks to process
            if let Some(todo_list) = todo_list {
                match todo_list.get_next_task().await {
                    Ok(Some(task)) => {
                        tracing::info!("Found task {} for agent {}", task.id, agent_name);
                        
                        // Process the task
                        let task_json = serde_json::to_string(&task)?;
                        let _ = process_todo_for_agent(
                            agent_registry,
                            &agent_name,
                            &task_json,
                            mqtt_client
                        ).await;
                        
                        // Short sleep to avoid hammering the system
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    },
                    Ok(None) => {
                        // No tasks to process, sleep for a while
                        tokio::time::sleep(check_interval).await;
                    },
                    Err(e) => {
                        tracing::error!("Failed to get next task for agent {}: {}", agent_name, e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn report_metrics(
    tasks_processed: u64,
    tasks_succeeded: u64,
    tasks_failed: u64,
    uptime: Duration,
    mqtt_client: &AsyncClient,
) -> Result<()> {
    let metrics = serde_json::json!({
        "tasks_processed": tasks_processed,
        "tasks_succeeded": tasks_succeeded,
        "tasks_failed": tasks_failed,
        "success_rate": if tasks_processed > 0 { (tasks_succeeded as f64 / tasks_processed as f64) * 100.0 } else { 0.0 },
        "uptime_seconds": uptime.as_secs(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    let metrics_topic = "metrics/todo_worker";
    mqtt_client.publish(metrics_topic, QoS::AtLeastOnce, false, metrics.to_string()).await?;
    tracing::info!("Published metrics: {}", metrics);
    
    Ok(())
}
