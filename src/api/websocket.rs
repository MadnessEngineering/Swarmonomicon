use std::sync::Arc;
use axum::{
    extract::ws::{WebSocket, Message as WsMessage},
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::{
    api::AppState,
    agents::{AgentRegistry, TransferService, GreeterAgent},
    types::{AgentConfig, Tool, Message},
};

#[cfg(feature = "haiku-agent")]
use crate::agents::HaikuAgent;

use tokio::sync::RwLock;
use crate::api::routes::AppError;
use axum::http::StatusCode;

const CHANNEL_SIZE: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Connect { agent: String },
    Message { content: String },
    Transfer { from: String, to: String },
    UpdateSession {
        instructions: String,
        tools: Vec<crate::types::Tool>,
        turn_detection: Option<TurnDetection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDetection {
    pub type_name: String,
    pub threshold: f32,
    pub prefix_padding_ms: u32,
    pub silence_duration_ms: u32,
    pub create_response: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Connected { agent: String },
    Message { content: String },
    Error { message: String },
    Transferred { from: String, to: String },
    SessionUpdated,
}

#[derive(Debug)]
pub enum WebSocketError {
    ConnectionError(String),
    AgentError(String),
}

impl From<WebSocketError> for StatusCode {
    fn from(error: WebSocketError) -> Self {
        match error {
            WebSocketError::ConnectionError(_) => StatusCode::BAD_REQUEST,
            WebSocketError::AgentError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    let mut registry = state.agents.write().await;
    let greeter_agent = GreeterAgent::new();
    registry.register("greeter".to_string(), Box::new(greeter_agent));
    drop(registry);

    while let Some(msg) = receiver.next().await {
        let msg = msg.map_err(|_| AppError::Status(StatusCode::BAD_REQUEST))?;

        match msg {
            Message::Text(text) => {
                let registry = state.agents.read().await;
                if let Some(agent) = registry.get("greeter") {
                    let response = agent.process_message(Message::new(text)).await
                        .map_err(|_| AppError::Status(StatusCode::INTERNAL_SERVER_ERROR))?;

                    sender.send(Message::Text(response.content))
                        .await
                        .map_err(|_| AppError::Status(StatusCode::INTERNAL_SERVER_ERROR))?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_client_message(msg: ClientMessage, state: Arc<AppState>) -> Result<ServerMessage, String> {
    match msg {
        ClientMessage::Connect { agent } => {
            let mut registry = state.agents.write().await;
            let greeter_agent = GreeterAgent::new();
            registry.register("greeter".to_string(), Box::new(greeter_agent));
            drop(registry);
            Ok(ServerMessage::Connected { agent })
        },
        ClientMessage::Message { content } => {
            let mut registry = state.agents.write().await;
            let greeter_agent = GreeterAgent::new();
            registry.register("greeter".to_string(), Box::new(greeter_agent));
            drop(registry);
            match greeter_agent.process_message(Message::new(content)).await {
                Ok(response) => Ok(ServerMessage::Message { content: response.content }),
                Err(e) => Err(e.to_string()),
            }
        },
        ClientMessage::Transfer { from, to } => {
            let mut registry = state.agents.write().await;
            let greeter_agent = GreeterAgent::new();
            registry.register("greeter".to_string(), Box::new(greeter_agent));
            drop(registry);
            Ok(ServerMessage::Transferred { from, to })
        },
        ClientMessage::UpdateSession { instructions, tools, turn_detection } => {
            // Handle session update
            Ok(ServerMessage::SessionUpdated)
        },
    }
}

pub async fn handle_websocket(
    mut ws: WebSocket,
    State(state): State<Arc<AppState>>,
) -> Result<(), WebSocketError> {
    let mut registry = state.agents.write().await;
    let config = AgentConfig {
        name: "greeter".to_string(),
        public_description: "Greeter agent".to_string(),
        instructions: "Greets users".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    };
    let greeter_agent = GreeterAgent::new(config);
    registry.register("greeter".to_string(), Box::new(greeter_agent));
    drop(registry);

    while let Some(msg) = ws.recv().await {
        let msg = msg.map_err(|e| WebSocketError::ConnectionError(e.to_string()))?;

        if let WsMessage::Text(text) = msg {
            let registry = state.agents.read().await;
            if let Some(agent) = registry.get_agent("greeter") {
                let mut agent = agent.write().await;
                let response = agent.process_message(Message::new(text)).await
                    .map_err(|e| WebSocketError::AgentError(e.to_string()))?;

                ws.send(WsMessage::from(response))
                    .await
                    .map_err(|e| WebSocketError::ConnectionError(e.to_string()))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::routes::default_agents;

    async fn setup_test_state() -> Arc<AppState> {
        let mut registry = AgentRegistry::new();

        // Add test agents
        let greeter_config = AgentConfig {
            name: "greeter".to_string(),
            public_description: "Test greeter".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let greeter_agent = GreeterAgent::new(greeter_config);
        registry.register("greeter".to_string(), Box::new(greeter_agent));

        let registry = Arc::new(RwLock::new(registry));
        Arc::new(AppState {
            agents: registry,
        })
    }

    #[tokio::test]
    async fn test_handle_connect() {
        let state = setup_test_state().await;
        let msg = ClientMessage::Connect {
            agent: "greeter".to_string(),
        };

        let response = handle_client_message(msg, state).await;
        match response {
            Ok(ServerMessage::Connected { agent }) => {
                assert_eq!(agent, "greeter");
            }
            _ => panic!("Expected Connected message"),
        }
    }

    #[tokio::test]
    async fn test_handle_message() {
        let state = setup_test_state().await;

        // First connect to an agent
        let connect_msg = ClientMessage::Connect {
            agent: "greeter".to_string(),
        };
        handle_client_message(connect_msg, state.clone()).await.expect("Failed to connect");

        // Then send a message
        let msg = ClientMessage::Message {
            content: "hi".to_string(),
        };

        let response = handle_client_message(msg, state).await;
        match response {
            Ok(ServerMessage::Message { content }) => {
                assert!(!content.is_empty());
            }
            _ => panic!("Expected Message response"),
        }
    }

    #[cfg(feature = "haiku-agent")]
    #[tokio::test]
    async fn test_handle_transfer() {
        let state = setup_test_state().await;

        let msg = ClientMessage::Transfer {
            from: "greeter".to_string(),
            to: "haiku".to_string(),
        };

        let response = handle_client_message(msg, state).await;
        match response {
            Ok(ServerMessage::Transferred { from, to }) => {
                assert_eq!(from, "greeter");
                assert_eq!(to, "haiku");
            }
            _ => panic!("Expected Transferred message"),
        }
    }
}
