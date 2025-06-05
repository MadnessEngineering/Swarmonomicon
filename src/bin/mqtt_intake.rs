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
use tokio::sync::broadcast;
use serde_json::json;
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: Option<TaskPriority>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectClassificationRequest {
    description: String,
    request_id: Option<String>,
    context: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectClassificationResponse {
    project_name: String,
    confidence: f64,
    request_id: Option<String>,
    reasoning: Option<String>,
}

// Maximum number of concurrent task processing
const MAX_CONCURRENT_TASKS: usize = 1;
// Maximum number of concurrent AI enhancements
const MAX_CONCURRENT_AI: usize = 1;
// Task metrics reporting interval
const METRICS_REPORTING_INTERVAL: u64 = 300;
// Project classification timeout
const PROJECT_CLASSIFICATION_TIMEOUT: u64 = 30;

// Simple metrics struct to track tasks
struct TaskMetrics {
    tasks_received: AtomicU64,
    tasks_processed: AtomicU64,
    tasks_failed: AtomicU64,
    project_classifications_requested: AtomicU64,
    project_classifications_successful: AtomicU64,
    start_time: Instant,
}

impl TaskMetrics {
    fn new() -> Self {
        Self {
            tasks_received: AtomicU64::new(0),
            tasks_processed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            project_classifications_requested: AtomicU64::new(0),
            project_classifications_successful: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn increment_received(&self) -> u64 {
        self.tasks_received.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_processed(&self) -> u64 {
        self.tasks_processed.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_failed(&self) -> u64 {
        self.tasks_failed.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_classification_requested(&self) -> u64 {
        self.project_classifications_requested.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_classification_successful(&self) -> u64 {
        self.project_classifications_successful.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn as_json(&self) -> serde_json::Value {
        let received = self.tasks_received.load(Ordering::SeqCst);
        let processed = self.tasks_processed.load(Ordering::SeqCst);
        let failed = self.tasks_failed.load(Ordering::SeqCst);
        let class_requested = self.project_classifications_requested.load(Ordering::SeqCst);
        let class_successful = self.project_classifications_successful.load(Ordering::SeqCst);
        let uptime_secs = self.start_time.elapsed().as_secs();

        json!({
            "tasks_received": received,
            "tasks_processed": processed,
            "tasks_failed": failed,
            "project_classifications_requested": class_requested,
            "project_classifications_successful": class_successful,
            "classification_success_rate": if class_requested > 0 { (class_successful as f64 / class_requested as f64) * 100.0 } else { 0.0 },
            "success_rate": if received > 0 { (processed as f64 / received as f64) * 100.0 } else { 0.0 },
            "uptime_seconds": uptime_secs,
            "tasks_per_minute": if uptime_secs > 0 { (received as f64 / uptime_secs as f64) * 60.0 } else { 0.0 }
        })
    }
}

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

    // Initialize metrics
    let metrics = Arc::new(TaskMetrics::new());

    let aws_ip = std::env::var("AWSIP").expect("AWSIP environment variable not set");
    let aws_port = std::env::var("AWSPORT").expect("AWSPORT environment variable not set").parse::<u16>().expect("AWSPORT must be a number");

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mqtt_intake", &aws_ip, aws_port);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    let client = Arc::new(client);
    tracing::info!("Connecting to MQTT broker at {}:{}", aws_ip, aws_port);

    // Subscribe to mcp/+ topic with retry logic
    for attempt in 1..=3 {
        match client.subscribe("mcp/+", QoS::ExactlyOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to mcp/+");
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

    // // Also subscribe to control topic
    // client.subscribe("mcp_server/control", QoS::ExactlyOnce).await
    //     .map_err(|e| anyhow!("Failed to subscribe to control topic: {}", e))?;

    tracing::info!("MCP Todo Server started. Listening for new tasks...");

    // Setup metrics reporting task
    let metrics_client = client.clone();
    let metrics_cloned = metrics.clone();
    let metrics_reporter = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(METRICS_REPORTING_INTERVAL));
        loop {
            interval.tick().await;

            // Report metrics
            let metrics_json = metrics_cloned.as_json();
            let _ = metrics_client.publish(
                "metrics/response/mqtt_intake",
                QoS::ExactlyOnce,
                false,
                metrics_json.to_string()
            ).await;
        }
    });

    // Set up graceful shutdown channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    // Set up ctrl-c handler
    let shutdown_tx_ctrl_c = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for ctrl-c: {}", e);
            return;
        }
        tracing::info!("Received shutdown signal, initiating graceful shutdown...");
        let _ = shutdown_tx_ctrl_c.send(());
    });

    // Main event loop
    loop {
        tokio::select! {
            // Check for shutdown signal
            result = shutdown_rx.recv() => {
                if result.is_ok() {
                    tracing::info!("Shutdown signal received, closing MQTT connection...");

                    // Publish final metrics and shutdown status
                    let shutdown_payload = json!({
                        "status": "shutdown",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "final_metrics": metrics.as_json()
                    }).to_string();

                    if let Err(e) = client.publish(
                        "response/mcp_server/status",
                        QoS::ExactlyOnce,
                        false,
                        shutdown_payload
                    ).await {
                        tracing::error!("Failed to publish shutdown status: {}", e);
                    }

                    // Disconnect from MQTT
                    if let Err(e) = client.disconnect().await {
                        tracing::error!("Error disconnecting from MQTT: {}", e);
                    }

                    // Allow time for final messages to be sent
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    tracing::info!("Graceful shutdown complete");
                    break;
                }
            }

            // Handle MQTT events
            event_result = event_loop.poll() => {
                match event_result {
                    Ok(notification) => {
                        if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                            let topic = publish.topic.clone();
                            let payload = String::from_utf8_lossy(&publish.payload).to_string();

                            // Handle control messages
                            if topic == "mcp_server/control" {
                                if let Ok(control_json) = serde_json::from_str::<serde_json::Value>(&payload) {
                                    if let Some(command) = control_json.get("command").and_then(|c| c.as_str()) {
                                        if command == "shutdown" {
                                            tracing::info!("Received shutdown command, initiating graceful shutdown...");
                                            let _ = shutdown_tx.send(());
                                            continue;
                                        } else if command == "status" {
                                            // Report current status
                                            let status_payload = json!({
                                                "status": "running",
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                "metrics": metrics.as_json()
                                            }).to_string();

                                            if let Err(e) = client.publish(
                                                "response/mcp_server/status",
                                                QoS::ExactlyOnce,
                                                false,
                                                status_payload
                                            ).await {
                                                tracing::error!("Failed to publish status: {}", e);
                                            }
                                            continue;
                                        }
                                    }
                                }
                            }

                            // Handle normal MCP task requests
                            if topic.starts_with("mcp/") {
                                tracing::info!("Received payload on {}: {}", topic, payload);

                                // Increment the task received counter
                                let task_count = metrics.increment_received();
                                tracing::debug!("Task count: {}", task_count);

                                // Clone necessary Arc's for the task
                                let todo_tool = todo_tool.clone();
                                let task_semaphore = task_semaphore.clone();
                                let ai_semaphore = ai_semaphore.clone();
                                let metrics = metrics.clone();
                                let client = client.clone();

                                // Spawn a new task to handle this request
                                tokio::spawn(async move {
                                    // Acquire task processing permit
                                    let _task_permit = match task_semaphore.acquire().await {
                                        Ok(permit) => permit,
                                        Err(e) => {
                                            tracing::error!("Failed to acquire task permit: {}", e);
                                            metrics.increment_failed();
                                            return;
                                        }
                                    };

                                    // Try to parse as McpTodoRequest, if fails treat as plain text
                                    let description = match serde_json::from_str::<McpTodoRequest>(&payload) {
                                        Ok(request) => request.description,
                                        Err(_) => payload,
                                    };

                                    let target_agent = topic.split('/').nth(1).unwrap_or("user");

                                    // Request project classification from project worker
                                    let request_id = Uuid::new_v4().to_string();
                                    let classification_request = ProjectClassificationRequest {
                                        description: description.clone(),
                                        request_id: Some(request_id.clone()),
                                        context: Some({
                                            let mut context = HashMap::new();
                                            context.insert("source".to_string(), "mqtt_intake".to_string());
                                            context.insert("target_agent".to_string(), target_agent.to_string());
                                            context
                                        }),
                                    };

                                    metrics.increment_classification_requested();

                                    // Subscribe to classification response topic with request ID
                                    let response_topic = format!("response/project/classify/{}", request_id);
                                    let subscription_client = client.clone();
                                    if let Err(e) = subscription_client.subscribe(&response_topic, QoS::ExactlyOnce).await {
                                        tracing::error!("Failed to subscribe to classification response topic: {}", e);
                                        metrics.increment_failed();
                                        return;
                                    }

                                    // Publish classification request
                                    let classification_payload = serde_json::to_string(&classification_request)
                                        .unwrap_or_else(|_| description.clone());

                                    if let Err(e) = client.publish(
                                        "project/classify",
                                        QoS::ExactlyOnce,
                                        false,
                                        classification_payload
                                    ).await {
                                        tracing::error!("Failed to publish classification request: {}", e);
                                        metrics.increment_failed();
                                        return;
                                    }

                                    // Wait for project classification response with timeout
                                    let project_name = match tokio::time::timeout(
                                        Duration::from_secs(PROJECT_CLASSIFICATION_TIMEOUT),
                                        wait_for_project_classification(&client, &request_id)
                                    ).await {
                                        Ok(Ok(response)) => {
                                            metrics.increment_classification_successful();
                                            tracing::info!("Received project classification: {} -> {}",
                                                description, response.project_name);
                                            response.project_name
                                        },
                                        Ok(Err(e)) => {
                                            tracing::warn!("Project classification failed: {}. Using default.", e);
                                            "madness_interactive".to_string()
                                        },
                                        Err(_) => {
                                            tracing::warn!("Project classification timed out. Using default.");
                                            "madness_interactive".to_string()
                                        }
                                    };

                                    // Add todo using TodoTool with classified project
                                    let mut params = HashMap::new();
                                    params.insert("command".to_string(), "add".to_string());
                                    params.insert("description".to_string(), description.clone());
                                    params.insert("context".to_string(), "mcp_server".to_string());
                                    params.insert("target_agent".to_string(), target_agent.to_string());
                                    params.insert("project".to_string(), project_name.clone());

                                    // Acquire AI enhancement permit before processing
                                    let _ai_permit = match ai_semaphore.acquire().await {
                                        Ok(permit) => permit,
                                        Err(e) => {
                                            tracing::error!("Failed to acquire AI permit: {}", e);
                                            metrics.increment_failed();
                                            return;
                                        }
                                    };

                                    match todo_tool.execute(params).await {
                                        Ok(result) => {
                                            tracing::info!("Successfully added todo: {} (project: {})", description, project_name);
                                            metrics.increment_processed();

                                            // Publish success response
                                            let response_topic = format!("response/{}/todo", target_agent);
                                            let response_payload = json!({
                                                "status": "success",
                                                "message": result,
                                                "project": project_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339()
                                            }).to_string();

                                            if let Err(e) = client.publish(
                                                response_topic,
                                                QoS::ExactlyOnce,
                                                false,
                                                response_payload
                                            ).await {
                                                tracing::error!("Failed to publish success response: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            tracing::error!("Failed to add todo: {}", e);
                                            metrics.increment_failed();

                                            // Publish error response
                                            let error_topic = format!("response/{}/error", target_agent);
                                            let error_payload = json!({
                                                "status": "error",
                                                "error": e.to_string(),
                                                "project": project_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339()
                                            }).to_string();

                                            if let Err(e) = client.publish(
                                                error_topic,
                                                QoS::ExactlyOnce,
                                                false,
                                                error_payload
                                            ).await {
                                                tracing::error!("Failed to publish error response: {}", e);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error from eventloop: {:?}", e);
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Wait for project classification response from project worker
async fn wait_for_project_classification(
    client: &Arc<AsyncClient>,
    request_id: &str
) -> Result<ProjectClassificationResponse> {
    use rumqttc::EventLoop;
    use std::sync::Arc;

    // Create a new event loop to listen specifically for our response
    let aws_ip = std::env::var("AWSIP").expect("AWSIP environment variable not set");
    let aws_port = std::env::var("AWSPORT")
        .expect("AWSPORT environment variable not set")
        .parse::<u16>()
        .expect("AWSPORT must be a number");

    let mut mqtt_options = MqttOptions::new(
        format!("classification_waiter_{}", request_id),
        &aws_ip,
        aws_port
    );
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);

    let (temp_client, mut temp_event_loop) = AsyncClient::new(mqtt_options, 10);

    // Subscribe to our specific response topic
    let response_topic = format!("response/project/classify/{}", request_id);
    temp_client.subscribe(&response_topic, QoS::ExactlyOnce).await?;

    // Also subscribe to general response topic as fallback
    temp_client.subscribe("response/project/classify", QoS::ExactlyOnce).await?;

    // Wait for response
    loop {
        match temp_event_loop.poll().await {
            Ok(Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                let topic = &publish.topic;
                let payload = String::from_utf8_lossy(&publish.payload);

                // Check if this is our response
                if topic == &response_topic ||
                   (topic == "response/project/classify" && payload.contains(request_id)) {

                    if let Ok(response) = serde_json::from_str::<ProjectClassificationResponse>(&payload) {
                        // Verify this is our request
                        if response.request_id.as_ref() == Some(&request_id.to_string()) ||
                           response.request_id.is_none() {
                            return Ok(response);
                        }
                    }
                }
            },
            Ok(_) => {
                // Continue listening for other events
                continue;
            },
            Err(e) => {
                return Err(anyhow!("Error waiting for classification response: {}", e));
            }
        }
    }
}
