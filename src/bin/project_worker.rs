use std::time::Duration;
use std::collections::HashMap;
use swarmonomicon::agents::project::{ProjectAgent, ProjectClassificationRequest, ProjectClassificationResponse};
use swarmonomicon::types::{AgentConfig, Message};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde::{Deserialize, Serialize};
use tokio::{time, sync::Semaphore};
use std::error::Error as StdError;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use serde_json::json;
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};

// Simple metrics struct to track project classification requests
struct ProjectMetrics {
    requests_received: AtomicU64,
    requests_processed: AtomicU64,
    requests_failed: AtomicU64,
    start_time: Instant,
}

impl ProjectMetrics {
    fn new() -> Self {
        Self {
            requests_received: AtomicU64::new(0),
            requests_processed: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn increment_received(&self) -> u64 {
        self.requests_received.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_processed(&self) -> u64 {
        self.requests_processed.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn increment_failed(&self) -> u64 {
        self.requests_failed.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn as_json(&self) -> serde_json::Value {
        let received = self.requests_received.load(Ordering::SeqCst);
        let processed = self.requests_processed.load(Ordering::SeqCst);
        let failed = self.requests_failed.load(Ordering::SeqCst);
        let uptime_secs = self.start_time.elapsed().as_secs();

        json!({
            "requests_received": received,
            "requests_processed": processed,
            "requests_failed": failed,
            "success_rate": if received > 0 { (processed as f64 / received as f64) * 100.0 } else { 0.0 },
            "uptime_seconds": uptime_secs,
            "requests_per_minute": if uptime_secs > 0 { (received as f64 / uptime_secs as f64) * 60.0 } else { 0.0 }
        })
    }
}

// Maximum number of concurrent requests
const MAX_CONCURRENT_REQUESTS: usize = 5;
// Metrics reporting interval
const METRICS_REPORTING_INTERVAL: u64 = 300; // 5 minutes

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize ProjectAgent
    let project_config = AgentConfig {
        name: "project-classifier".to_string(),
        public_description: "AI-powered project classification agent".to_string(),
        instructions: "Classify incoming tasks to determine which project they belong to and perform background project maintenance".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    };

    let project_agent = Arc::new(ProjectAgent::new(project_config).await
        .map_err(|e| anyhow!("Failed to initialize ProjectAgent: {}", e))?);

    // Create semaphore for rate limiting
    let request_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    // Initialize metrics
    let metrics = Arc::new(ProjectMetrics::new());

    let aws_ip = std::env::var("AWSIP").expect("AWSIP environment variable not set");
    let aws_port = std::env::var("AWSPORT")
        .expect("AWSPORT environment variable not set")
        .parse::<u16>()
        .expect("AWSPORT must be a number");

    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("project_worker", &aws_ip, aws_port);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    let client = Arc::new(client);
    tracing::info!("Connecting to MQTT broker at {}:{}", aws_ip, aws_port);

    // Subscribe to project classification topic
    for attempt in 1..=3 {
        match client.subscribe("project/classify", QoS::ExactlyOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to project/classify");
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

    // Also subscribe to control topic
    client.subscribe("project_worker/control", QoS::ExactlyOnce).await
        .map_err(|e| anyhow!("Failed to subscribe to control topic: {}", e))?;

    tracing::info!("Project Worker started. Listening for classification requests...");

    // Setup metrics reporting task
    let metrics_client = client.clone();
    let metrics_cloned = metrics.clone();
    let _metrics_reporter = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(METRICS_REPORTING_INTERVAL));
        loop {
            interval.tick().await;

            // Report metrics
            let metrics_json = metrics_cloned.as_json();
            let _ = metrics_client.publish(
                "metrics/response/project_worker",
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
                        "response/project_worker/status",
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
                            if topic == "project_worker/control" {
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
                                                "response/project_worker/status",
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

                            // Handle project classification requests
                            if topic == "project/classify" {
                                tracing::info!("Received classification request: {}", payload);

                                // Increment the request received counter
                                let request_count = metrics.increment_received();
                                tracing::debug!("Request count: {}", request_count);

                                // Clone necessary Arc's for the task
                                let project_agent = project_agent.clone();
                                let request_semaphore = request_semaphore.clone();
                                let metrics = metrics.clone();
                                let client = client.clone();

                                // Spawn a new task to handle this request
                                tokio::spawn(async move {
                                    // Acquire request processing permit
                                    let _request_permit = match request_semaphore.acquire().await {
                                        Ok(permit) => permit,
                                        Err(e) => {
                                            tracing::error!("Failed to acquire request permit: {}", e);
                                            metrics.increment_failed();
                                            return;
                                        }
                                    };

                                    // Try to parse as ProjectClassificationRequest
                                    let classification_request = match serde_json::from_str::<ProjectClassificationRequest>(&payload) {
                                        Ok(request) => request,
                                        Err(_) => {
                                            // If parsing fails, create a simple request with the payload as description
                                            ProjectClassificationRequest {
                                                description: payload,
                                                request_id: None,
                                                context: None,
                                            }
                                        }
                                    };

                                    // Process the classification request
                                    match project_agent.classify_project(classification_request.clone()).await {
                                        Ok(response) => {
                                            tracing::info!("Successfully classified project: {} -> {}", 
                                                classification_request.description, response.project_name);
                                            metrics.increment_processed();

                                            // Publish success response
                                            let response_topic = if let Some(request_id) = &response.request_id {
                                                format!("response/project/classify/{}", request_id)
                                            } else {
                                                "response/project/classify".to_string()
                                            };

                                            let response_payload = serde_json::to_string(&response).unwrap_or_else(|_| {
                                                json!({
                                                    "project_name": response.project_name,
                                                    "confidence": response.confidence
                                                }).to_string()
                                            });

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
                                            tracing::error!("Failed to classify project: {}", e);
                                            metrics.increment_failed();

                                            // Publish error response
                                            let error_topic = "response/project/classify/error";
                                            let error_payload = json!({
                                                "status": "error",
                                                "error": e.to_string(),
                                                "request_id": classification_request.request_id,
                                                "fallback_project": "madness_interactive",
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
