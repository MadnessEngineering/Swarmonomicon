use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tokio::sync::RwLock;
use crate::{
    agents::{AgentRegistry, TransferService},
    types::Agent,
};

mod models;
mod routes;
mod websocket;

pub use models::*;
pub use routes::*;
pub use websocket::*;

pub struct AppState {
    pub transfer_service: Arc<RwLock<TransferService>>,
    pub agents: Arc<RwLock<AgentRegistry>>,
}

impl AppState {
    pub fn new(transfer_service: Arc<RwLock<TransferService>>) -> Self {
        Self {
            transfer_service,
            agents: Arc::new(RwLock::new(AgentRegistry::new()))
        }
    }
}

pub async fn create_app_state() -> Arc<AppState> {
    let registry = AgentRegistry::create_default_agents(routes::default_agents()).await.unwrap();
    let registry = Arc::new(RwLock::new(registry));
    let transfer_service = Arc::new(RwLock::new(TransferService::new(registry.clone())));

    Arc::new(AppState::new(transfer_service))
}

pub async fn serve(addr: SocketAddr, transfer_service: Arc<RwLock<TransferService>>) {
    let registry = AgentRegistry::create_default_agents(routes::default_agents()).await.unwrap();
    let app_state = Arc::new(AppState {
        transfer_service,
        agents: Arc::new(RwLock::new(registry)),
    });

    let app = Router::new()
        .route("/", get(routes::index))
        .route("/api/agents", get(routes::list_agents))
        .route("/api/agents/:name", get(routes::get_agent))
        .route("/api/agents/:name/message", post(routes::process_message))
        .route("/api/agents/:name/send", post(routes::send_message))
        .route("/api/agents/:name/tasks", get(routes::get_tasks))
        .route("/api/agents/:name/tasks", post(routes::add_task))
        .route("/api/agents/:name/tasks/:task_id", get(routes::get_task))
        .route("/ws", get(websocket::websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    println!("Server running on {}", addr);
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agents", get(routes::list_agents))
        .route("/agents/:agent_name/message", post(routes::send_message))
        .route("/ws", get(websocket::websocket_handler))
        .with_state(state)
}
