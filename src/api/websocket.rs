use std::sync::Arc;
use axum::{
    extract::ws::{WebSocket, Message as WsMessage},
    extract::{State, WebSocketUpgrade},
    response::Response,
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
use serde_json::{json, Value};
use anyhow::{Result, anyhow};

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

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Text(content) = msg {
            let response = match serde_json::from_str::<ClientMessage>(&content) {
                Ok(client_msg) => {
                    match handle_client_message(client_msg, state.clone()).await {
                        Ok(server_msg) => {
                            match serde_json::to_string(&server_msg) {
                                Ok(json) => WsMessage::Text(json),
                                Err(_) => WsMessage::Text("Error serializing response".to_string()),
                            }
                        },
                        Err(e) => WsMessage::Text(format!("Error: {}", e)),
                    }
                },
                Err(_) => WsMessage::Text("Invalid message format".to_string()),
            };

            if sender.send(response).await.is_err() {
                break;
            }
        }
    }
}

async fn handle_client_message(msg: ClientMessage, state: Arc<AppState>) -> Result<ServerMessage, String> {
    match msg {
        ClientMessage::Connect { agent } => {
            let mut transfer_service = state.transfer_service.write().await;
            transfer_service.set_current_agent_name(&agent).await.map_err(|e| e.to_string())?;
            Ok(ServerMessage::Connected { agent })
        },
        ClientMessage::Message { content } => {
            let transfer_service = state.transfer_service.read().await;
            match transfer_service.process_message(Message::new(content)).await {
                Ok(response) => Ok(ServerMessage::Message { content: response.content }),
                Err(e) => Err(e.to_string()),
            }
        },
        ClientMessage::Transfer { from, to } => {
            let mut transfer_service = state.transfer_service.write().await;
            transfer_service.set_current_agent_name(&to).await.map_err(|e| e.to_string())?;
            Ok(ServerMessage::Transferred { from, to })
        },
        ClientMessage::UpdateSession { instructions, tools, turn_detection } => {
            // Handle session update
            Ok(ServerMessage::SessionUpdated)
        },
    }
}

pub async fn handle_websocket(
    mut socket: WebSocket,
    transfer_service: Arc<RwLock<TransferService>>,
) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                WsMessage::Text(text) => {
                    let response = handle_message(text, &transfer_service).await;
                    match response {
                        Ok(response) => {
                            if let Err(e) = socket.send(WsMessage::Text(response)).await {
                                eprintln!("Error sending response: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            let error_response = json!({
                                "error": e.to_string()
                            }).to_string();
                            if let Err(e) = socket.send(WsMessage::Text(error_response)).await {
                                eprintln!("Error sending error response: {}", e);
                                break;
                            }
                        }
                    }
                }
                WsMessage::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

async fn handle_message(text: String, transfer_service: &Arc<RwLock<TransferService>>) -> Result<String> {
    let mut transfer_service = transfer_service.write().await;
    let data: Value = serde_json::from_str(&text)?;

    match data["type"].as_str() {
        Some("message") => {
            let content = data["content"].as_str()
                .ok_or_else(|| anyhow!("Missing content field"))?;

            let current_agent = transfer_service.get_current_agent_name().await?;
            let agent = transfer_service.get_agent(&current_agent).await?;
            let response = agent.process_message(Message::new(content.to_string())).await?;

            Ok(json!({
                "type": "response",
                "content": response.content,
                "metadata": response.metadata
            }).to_string())
        }
        Some("transfer") => {
            let to = data["to"].as_str()
                .ok_or_else(|| anyhow!("Missing 'to' field"))?;

            let from = transfer_service.get_current_agent_name().await?;
            let message = Message::new("Transfer request".to_string());
            let response = transfer_service.transfer(&from, to, message).await?;
            transfer_service.set_current_agent_name(to).await?;

            Ok(json!({
                "type": "transfer",
                "from": from,
                "to": to,
                "response": response
            }).to_string())
        }
        Some(t) => Err(anyhow!("Unknown message type: {}", t)),
        None => Err(anyhow!("Missing message type")),
    }
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
        registry.register("greeter".to_string(), Box::new(greeter_agent)).await.expect("Failed to register greeter agent");

        let registry = Arc::new(RwLock::new(registry));
        Arc::new(AppState {
            transfer_service: Arc::new(RwLock::new(TransferService::new(registry.clone()))),
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
