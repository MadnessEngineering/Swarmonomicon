mod routes;
mod websocket;

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use axum::{
    Router,
    routing::{get, post},
    response::IntoResponse,
    http::StatusCode,
};
use tower_http::{
    trace::TraceLayer,
    services::ServeDir,
};
use crate::agents::{AgentRegistry, TransferService};
use crate::Result;

pub struct AppState {
    transfer_service: Arc<RwLock<TransferService>>,
}

pub async fn serve(port: u16) -> Result<()> {
    let registry = AgentRegistry::create_default_agents(routes::default_agents())?;
    let registry = Arc::new(RwLock::new(registry));
    let transfer_service = Arc::new(RwLock::new(TransferService::new(registry)));

    let app_state = AppState {
        transfer_service: transfer_service.clone(),
    };

    let app = Router::new()
        .route("/", get(routes::index))
        .route("/api/agents", get(routes::list_agents))
        .route("/api/agents/:name", get(routes::get_agent))
        .route("/api/agents/:name/message", post(routes::send_message))
        .route("/ws", get(websocket::handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(app_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server running on http://{}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(&addr).await?,
        app.into_make_service(),
    )
    .await?;

    Ok(())
}

pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
