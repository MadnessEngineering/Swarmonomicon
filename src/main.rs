use std::net::SocketAddr;
use swarmonomicon::api::{serve, create_app_state};

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();

    // Set up the server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Create app state
    let app_state = create_app_state().await;

    // Run the server
    println!("Starting server on {}", addr);
    serve(addr, app_state.transfer_service.clone()).await;
} 
