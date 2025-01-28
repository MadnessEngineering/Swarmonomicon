use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    api::AppState,
    types::{Message, AgentConfig, Agent, AgentInfo},
    agents::AgentRegistry,
};

pub async fn index() -> Response {
    "Welcome to the Swarmonomicon API".into_response()
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    name: String,
    description: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    content: String,
}

pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut agents = Vec::new();
    for agent in state.agents.values() {
        if let Ok(config) = agent.get_config().await {
            agents.push(AgentInfo {
                name: config.name,
                description: config.public_description,
                tools: config.tools,
                downstream_agents: config.downstream_agents,
            });
        }
    }
    Json(agents)
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let transfer_service = state.transfer_service.read().await;
    let registry = transfer_service.get_registry().read().await;
    match registry.get(&name) {
        Some(agent) => Json(AgentInfo {
            name: agent.get_config().await.unwrap().name,
            description: agent.get_config().await.unwrap().public_description,
            tools: agent.get_config().await.unwrap().tools,
            downstream_agents: agent.get_config().await.unwrap().downstream_agents,
        }).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Agent '{}' not found", name)
            }))
        ).into_response(),
    }
}

pub async fn process_message(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(message): Json<Message>,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(agent) = state.agents.get(&agent_name) {
        match agent.process_message(message).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Response {
    let transfer_service = state.transfer_service.read().await;
    let registry = transfer_service.get_registry().read().await;
    match registry.get(&name) {
        Some(_) => {
            let response = Message::new("Connected to agent system".to_string());
            Json(response).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Agent '{}' not found", name)
            }))
        ).into_response(),
    }
}

pub fn default_agents() -> Vec<AgentConfig> {
    // vec![ restore default later ?
    //     AgentConfig {
    //         name: "greeter".to_string(),
    //         public_description: "Agent that greets the user.".to_string(),
    //         instructions: "Please greet the user to the Swarmonomicon project.".to_string(),
    //         tools: Vec::new(),
    //         downstream_agents: vec!["haiku".to_string()],
    //         personality: None,
    //         state_machine: None,
    //     }
    // ]
    let mut agents = Vec::new();

    #[cfg(feature = "greeter-agent")]
    agents.push(AgentConfig {
        name: "greeter".to_string(),
        public_description: "Agent that greets the user.".to_string(),
        instructions: "Please greet the user and ask them if they'd like a Haiku. If yes, transfer them to the 'haiku' agent.".to_string(),
        tools: Vec::new(),
        downstream_agents: vec!["haiku".to_string()],
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "haiku-agent")]
    agents.push(AgentConfig {
        name: "haiku".to_string(),
        public_description: "Agent that creates haikus.".to_string(),
        instructions: "Create haikus based on user input.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "git-agent")]
    agents.push(AgentConfig {
        name: "git".to_string(),
        public_description: "Agent that helps with git operations.".to_string(),
        instructions: "Help users with git operations like commit, branch, merge etc.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "project-init-agent")]
    agents.push(AgentConfig {
        name: "project-init".to_string(),
        public_description: "Agent that helps initialize new projects.".to_string(),
        instructions: "Help users create new projects with proper structure and configuration.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "browser-agent")]
    agents.push(AgentConfig {
        name: "browser".to_string(),
        public_description: "Agent that controls browser automation.".to_string(),
        instructions: "Help users with browser automation tasks.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    agents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_agents() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });
        let response = list_agents(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_agent() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });
        let response = get_agent(State(state.clone()), Path("unknown".to_string())).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_send_message() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });
        let request = MessageRequest {
            content: "Hello".to_string(),
        };
        let response = send_message(
            State(state),
            Path("unknown".to_string()),
            Json(request),
        ).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
