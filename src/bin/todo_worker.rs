use std::time::Duration;
use swarmonomicon::agents::{self, UserAgent};
use swarmonomicon::types::AgentConfig;

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
        downstream_agents: vec!["git".to_string(), "project".to_string(), "haiku".to_string()],
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
                name: "project".to_string(),
                public_description: "Initializes projects".to_string(),
                instructions: "Set up new project structures".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "haiku".to_string(),
                public_description: "Creates documentation".to_string(),
                instructions: "Create haiku-style documentation".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
        ])?;
    }

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
                        match agent.process_message(&todo.description).await {
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
