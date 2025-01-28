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

pub async fn list_agents(State(state): State<Arc<AppState>>) -> Response {
    let transfer_service = state.transfer_service.read().await;
    let registry = transfer_service.get_registry().read().await;
    let agents = registry.get_all_agents()
        .iter()
        .map(|agent| AgentInfo {
            name: agent.get_config().await.unwrap().name,
            description: agent.get_config().await.unwrap().public_description,
            tools: agent.get_config().await.unwrap().tools,
            downstream_agents: agent.get_config().await.unwrap().downstream_agents,
        })
        .collect::<Vec<_>>();

    Json(agents).into_response()
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
    vec![
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Agent that greets the user.".to_string(),
            instructions: "Please greet the user and ask them if they'd like a Haiku. If yes, transfer them to the 'haiku' agent.".to_string(),
            tools: Vec::new(),
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        }
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_agents() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry)));
        let state = Arc::new(AppState {
            transfer_service,
        });
        let response = list_agents(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_agent() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry)));
        let state = Arc::new(AppState {
            transfer_service,
        });
        let response = get_agent(State(state.clone()), Path("unknown".to_string())).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_send_message() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry)));
        let state = Arc::new(AppState {
            transfer_service,
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
