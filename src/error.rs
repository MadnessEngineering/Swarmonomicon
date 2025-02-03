use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("State error: {0}")]
    State(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SwarmError {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("State error: {0}")]
    State(String),
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Agent(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Agent(msg)
    }
}

impl From<SwarmError> for Error {
    fn from(err: SwarmError) -> Self {
        match err {
            SwarmError::Agent(msg) => Error::Agent(msg),
            SwarmError::Tool(msg) => Error::Tool(msg),
            SwarmError::State(msg) => Error::State(msg),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Agent(err.to_string())
    }
}
