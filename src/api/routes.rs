use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{
    types::{AgentConfig, Message},
    Result,
};
use super::AppState;

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    name: String,
    description: String,
    downstream_agents: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    content: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

pub async fn index() -> impl IntoResponse {
    "Welcome to Swarmonomicon API"
}

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AgentResponse>>> {
    let transfer_service = state.transfer_service.read().await;
    let registry = transfer_service.get_registry().read().await;
    
    let agents = registry
        .get_all_agents()
        .iter()
        .map(|agent| AgentResponse {
            name: agent.get_config().name.clone(),
            description: agent.get_config().public_description.clone(),
            downstream_agents: agent.get_config().downstream_agents.clone(),
        })
        .collect();

    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<AgentResponse>> {
    let transfer_service = state.transfer_service.read().await;
    let registry = transfer_service.get_registry().read().await;
    
    if let Some(agent) = registry.get(&name) {
        Ok(Json(AgentResponse {
            name: agent.get_config().name.clone(),
            description: agent.get_config().public_description.clone(),
            downstream_agents: agent.get_config().downstream_agents.clone(),
        }))
    } else {
        Err(format!("Agent {} not found", name).into())
    }
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<Message>> {
    let mut transfer_service = state.transfer_service.write().await;
    
    // Set current agent if not set
    if transfer_service.get_current_agent().is_none() {
        transfer_service.transfer("greeter", &name).await?;
    }

    // Process message
    let response = transfer_service.process_message(&request.content).await?;
    Ok(Json(response))
}

pub fn default_agents() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Greets users and offers to write haikus".to_string(),
            instructions: "Greet users and offer to write haikus".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        },
        AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus about any topic".to_string(),
            instructions: "Create haikus based on user topics".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentRegistry, TransferService};
    use tokio::sync::RwLock;

    async fn setup_test_state() -> Arc<AppState> {
        let registry = AgentRegistry::create_default_agents(default_agents()).unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let transfer_service = Arc::new(RwLock::new(TransferService::new(registry)));
        
        Arc::new(AppState {
            transfer_service,
        })
    }

    #[tokio::test]
    async fn test_list_agents() {
        let state = setup_test_state().await;
        let response = list_agents(State(state)).await.unwrap();
        let agents = response.0;
        
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].name, "greeter");
        assert_eq!(agents[1].name, "haiku");
    }

    #[tokio::test]
    async fn test_get_agent() {
        let state = setup_test_state().await;
        let response = get_agent(State(state), Path("greeter".to_string())).await.unwrap();
        let agent = response.0;
        
        assert_eq!(agent.name, "greeter");
        assert!(agent.downstream_agents.contains(&"haiku".to_string()));
    }

    #[tokio::test]
    async fn test_send_message() {
        let state = setup_test_state().await;
        let request = MessageRequest {
            content: "hi".to_string(),
        };
        
        let response = send_message(
            State(state),
            Path("greeter".to_string()),
            Json(request),
        ).await.unwrap();
        
        let message = response.0;
        assert!(message.content.contains("haiku"));
    }
} 
