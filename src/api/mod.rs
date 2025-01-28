use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tokio::sync::RwLock;
use crate::agents::TransferService;
use std::collections::HashMap;
use crate::agents::Agent;
use crate::{
    agents::{AgentRegistry, TransferService},
    api::routes::{default_agents, websocket_handler},
    types::Agent,
    AppState,
};

pub mod routes;
pub mod websocket;

pub struct AppState {
    pub transfer_service: Arc<RwLock<TransferService>>,
    pub agents: Arc<RwLock<AgentRegistry>>,
}

impl AppState {
    pub fn new(transfer_service: Arc<RwLock<TransferService>>) -> Self {
        Self { transfer_service, agents: Arc::new(RwLock::new(AgentRegistry::new())) }
    }
}

pub async fn create_app_state() -> Arc<AppState> {
    use crate::agents::AgentRegistry;
    use crate::api::routes::default_agents;

    let registry = AgentRegistry::create_default_agents(default_agents()).await.unwrap();
    let registry = Arc::new(RwLock::new(registry));
    let transfer_service = Arc::new(RwLock::new(TransferService::new(registry)));

    Arc::new(AppState::new(transfer_service))
}

pub async fn serve(addr: SocketAddr, transfer_service: Arc<RwLock<TransferService>>) {
    let registry = AgentRegistry::create_default_agents(default_agents()).await.unwrap();
    let app_state = Arc::new(AppState {
        transfer_service,
        agents: Arc::new(RwLock::new(registry)),
    });

    let app = Router::new()
        .route("/", get(routes::index))
        .route("/api/agents", get(routes::list_agents))
        .route("/api/agents/:name", get(routes::get_agent))
        .route("/api/agents/:name/message", post(routes::send_message))
        .route("/ws", get(websocket::websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    println!("Server starting on {}", addr);
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

pub async fn create_router() -> Router {
    let registry = Arc::new(RwLock::new(AgentRegistry::new()));
    let transfer_service = Arc::new(RwLock::new(TransferService::new(registry.clone())));
    
    let app_state = Arc::new(AppState {
        transfer_service,
        agents: registry,
    });

    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(app_state)
}
