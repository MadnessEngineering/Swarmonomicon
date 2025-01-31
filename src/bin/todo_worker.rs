use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::{AgentConfig, Message};
use swarmonomicon::Agent;
use rumqttc::{MqttOptions, Client, Event, QoS};
use tokio::task;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> swarmonomicon::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create the user agent with a specific state file
    let user_config = AgentConfig {
        name: "user".to_string(),
        public_description: "Manages and coordinates tasks".to_string(),
        instructions: "Process and coordinate tasks between agents".to_string(),
        tools: vec![],
        downstream_agents: vec![
            "git".to_string(),
            "project-init".to_string(),
            "haiku".to_string(),
            "browser".to_string(),
            "greeter".to_string()
        ],
        personality: None,
        state_machine: None,
    };

    let mut user_agent = UserAgent::with_state_file(user_config, "todo_state.json")?;

    // Initialize the global registry with default agents
    let registry = agents::GLOBAL_REGISTRY.clone();
    {
        let mut registry = registry.write().await;
        *registry = agents::AgentRegistry::create_default_agents(vec![
            AgentConfig {
                name: "git".to_string(),
                public_description: "Handles git operations".to_string(),
                instructions: "Manage git repositories and operations".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "project-init".to_string(),
                public_description: "Initializes projects".to_string(),
                instructions: "Set up new project structures and configurations".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "haiku".to_string(),
                public_description: "Creates documentation and creative content".to_string(),
                instructions: "Create haiku-style documentation and creative writing".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "browser".to_string(),
                public_description: "Controls browser automation".to_string(),
                instructions: "Help users with browser automation tasks".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "greeter".to_string(),
                public_description: "Handles user interaction and routing".to_string(),
                instructions: "Greet users and help route them to appropriate agents".to_string(),
                tools: vec![],
                downstream_agents: vec!["haiku".to_string()],
                personality: None,
                state_machine: None,
            },
        ]).await?;
    }

    // Start the MQTT listener in a separate task
    let mqtt_options = MqttOptions::new("todo_worker", "broker.hivemq.com", 1883);
    let (mqtt_client, mut event_loop) = Client::new(mqtt_options, 10);
    let topic = "todos";

    task::spawn(async move {
        mqtt_client.subscribe(topic, QoS::AtMostOnce).await.unwrap();
        while let Ok(event) = event_loop.poll().await {
            match event {
                Event::Incoming(rumqttc::Packet::Publish(publish)) => {
                    let description = String::from_utf8_lossy(&publish.payload).to_string();
                    let todo_name = publish.topic.clone();
                    // Create a new todo based on the MQTT message
                    let params = HashMap::new();
                    params.insert("command".to_string(), "add".to_string());
                    params.insert("description".to_string(), description);
                    // Execute the command to add the todo
                    let _ = user_agent.execute(params).await;
                }
                _ => {}
            }
        }
    });

    tracing::info!("Todo worker started. Checking for tasks every 2 minutes...");

    loop {
        // Check if we should process (at least 2 minutes since last process)
        let should_process = match user_agent.get_last_processed() {
            Some(last) => {
                let elapsed = chrono::Utc::now() - last;
                elapsed.num_seconds() >= 120
            }
            None => true,
        };

        if should_process {
            if let Some((index, todo)) = user_agent.get_next_pending_todo() {
                tracing::info!("Processing todo: {}", todo.description);

                // Try to determine the agent
                if let Ok(Some(agent_name)) = user_agent.determine_next_agent(todo).await {
                    // Get the agent from the global registry
                    let mut registry = registry.write().await;
                    if let Some(agent) = registry.get_mut(&agent_name) {
                        // Try to process the todo with the agent
                        match agent.process_message(Message::new(todo.description.clone())).await {
                            Ok(_) => {
                                tracing::info!("Task completed successfully");
                                user_agent.mark_todo_completed(index)?;
                            }
                            Err(e) => {
                                tracing::error!("Failed to process task: {}", e);
                                user_agent.mark_todo_failed(index, Some(e.to_string()))?;
                            }
                        }
                    } else {
                        tracing::error!("Agent {} not found in registry", agent_name);
                        user_agent.mark_todo_failed(index, Some(format!("Agent {} not found", agent_name)))?;
                    }
                } else {
                    tracing::error!("Could not determine agent for task");
                    user_agent.mark_todo_failed(index, Some("Could not determine appropriate agent".to_string()))?;
                }
            }

            user_agent.update_last_processed()?;
        }

        // Sleep for a bit to avoid busy waiting
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
