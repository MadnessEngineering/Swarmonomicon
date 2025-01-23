use std::sync::Arc;
use axum::{
    extract::ws::{WebSocket, Message as WsMessage},
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use super::AppState;

const CHANNEL_SIZE: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Connect { agent: String },
    Message { content: String },
    Transfer { to: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Connected { agent: String },
    Message { content: String },
    Error { message: String },
    Transferred { from: String, to: String },
}

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, _rx) = broadcast::channel(CHANNEL_SIZE);
    let tx2 = tx.clone();

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    let response = handle_client_message(client_msg, state.clone()).await;
                    if let Ok(response) = serde_json::to_string(&response) {
                        let _ = tx.send(WsMessage::Text(response));
                    }
                }
            }
        }
    });

    // Handle outgoing messages
    let mut send_task = tokio::spawn(async move {
        let mut rx = tx2.subscribe();
        while let Ok(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut recv_task) => send_task.abort(),
        _ = (&mut send_task) => recv_task.abort(),
    };
}

async fn handle_client_message(
    msg: ClientMessage,
    state: Arc<AppState>,
) -> ServerMessage {
    let mut transfer_service = state.transfer_service.write().await;

    match msg {
        ClientMessage::Connect { agent } => {
            match transfer_service.transfer("greeter", &agent).await {
                Ok(_) => ServerMessage::Connected { agent },
                Err(e) => ServerMessage::Error {
                    message: e.to_string(),
                },
            }
        }
        ClientMessage::Message { content } => {
            match transfer_service.process_message(&content).await {
                Ok(response) => ServerMessage::Message {
                    content: response.content,
                },
                Err(e) => ServerMessage::Error {
                    message: e.to_string(),
                },
            }
        }
        ClientMessage::Transfer { to } => {
            let from = transfer_service.get_current_agent().map(|s| s.to_string());
            if let Some(from) = from {
                match transfer_service.transfer(&from, &to).await {
                    Ok(_) => ServerMessage::Transferred {
                        from,
                        to,
                    },
                    Err(e) => ServerMessage::Error {
                        message: e.to_string(),
                    },
                }
            } else {
                ServerMessage::Error {
                    message: "No current agent".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentRegistry, TransferService};
    use crate::api::routes::default_agents;
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
