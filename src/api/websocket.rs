use std::sync::Arc;
use axum::{
    extract::ws::{WebSocket, Message as WsMessage, WebSocketUpgrade},
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::{
    api::AppState,
    agents::{AgentRegistry, TransferService, GreeterAgent, HaikuAgent},
    types::{AgentConfig, Tool, Message},
};
use tokio::sync::RwLock;

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
    state: Arc<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Text(content) = msg {
            let transfer_service = state.transfer_service.read().await;
            match transfer_service.process_message(Message::new(content.to_string())).await {
                Ok(response) => {
                    if let Err(e) = sender.send(WsMessage::Text(response.content)).await {
                        eprintln!("Error sending response: {}", e);
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("Error processing message: {}", e);
                    if let Err(e) = sender.send(WsMessage::Text(format!("Error: {}", e))).await {
                        eprintln!("Error sending error response: {}", e);
                        return;
                    }
                }
            }
        }
    }
}

async fn handle_client_message(
    msg: ClientMessage,
    state: Arc<AppState>,
) -> ServerMessage {
    let mut transfer_service = state.transfer_service.write().await;

    match msg {
        ClientMessage::Connect { agent } => {
            // Set the current agent using the public method
            transfer_service.set_current_agent(agent.clone());
            ServerMessage::Connected { agent }
        }
        ClientMessage::Message { content } => {
            match transfer_service.process_message(Message::new(content)).await {
                Ok(response) => {
                    if let Err(e) = tx.send(Message::Text(response.content)).await {
                        eprintln!("Error sending response: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error processing message: {}", e);
                    if let Err(e) = tx.send(Message::Text(format!("Error: {}", e))).await {
                        eprintln!("Error sending error response: {}", e);
                        break;
                    }
                }
            }
        }
        ClientMessage::Transfer { from, to } => {
            match transfer_service.transfer(&from, &to).await {
                Ok(_) => ServerMessage::Transferred { from, to },
                Err(e) => ServerMessage::Error {
                    message: e.to_string(),
                },
            }
        }
        ClientMessage::UpdateSession { instructions: _, tools: _, turn_detection: _ } => {
            // TODO: Implement session update logic
            ServerMessage::SessionUpdated
        }
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
            public_description: "Greets users".to_string(),
            instructions: "Greet the user".to_string(),
            tools: Vec::new(),
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        };

        let haiku_config = AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus".to_string(),
            instructions: "Create haikus".to_string(),
            tools: Vec::new(),
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        registry.register(GreeterAgent::new(greeter_config)).expect("Failed to register greeter agent");
        registry.register(HaikuAgent::new(haiku_config)).expect("Failed to register haiku agent");

        let registry = Arc::new(RwLock::new(registry));
        let transfer_service = Arc::new(RwLock::new(TransferService::new(registry)));

        Arc::new(AppState {
            transfer_service,
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
            ServerMessage::Connected { agent } => {
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
        handle_client_message(connect_msg, state.clone()).await;

        // Then send a message
        let msg = ClientMessage::Message {
            content: "hi".to_string(),
        };

        let response = handle_client_message(msg, state).await;
        match response {
            ServerMessage::Message { content } => {
                assert!(content.contains("haiku"));
            }
            _ => panic!("Expected Message response"),
        }
    }

    #[tokio::test]
    async fn test_handle_transfer() {
        let state = setup_test_state().await;

        // First connect to greeter
        let connect_msg = ClientMessage::Connect {
            agent: "greeter".to_string(),
        };
        handle_client_message(connect_msg, state.clone()).await;

        // Then transfer to haiku
        let msg = ClientMessage::Transfer {
            from: "greeter".to_string(),
            to: "haiku".to_string(),
        };

        let response = handle_client_message(msg, state).await;
        match response {
            ServerMessage::Transferred { from, to } => {
                assert_eq!(from, "greeter");
                assert_eq!(to, "haiku");
            }
            _ => panic!("Expected Transferred message"),
        }
    }
}
