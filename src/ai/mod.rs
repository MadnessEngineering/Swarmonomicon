use std::collections::HashMap;
use anyhow::Result;

mod goose;
mod local;

pub use goose::GooseClient;
pub use local::LocalAiClient;

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String>;
}

// Re-export the default client based on feature flags
#[cfg(feature = "goose")]
pub type DefaultAiClient = GooseClient;

#[cfg(not(feature = "goose"))]
pub type DefaultAiClient = LocalAiClient;

// Helper function to create a new AI client
pub fn new_ai_client() -> DefaultAiClient {
    DefaultAiClient::new()
}

// Deprecated: Use new_ai_client() instead
#[deprecated(since = "0.1.0", note = "please use `new_ai_client()` instead")]
pub use self::local::LocalAiClient as AiClient;
