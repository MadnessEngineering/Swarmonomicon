#![allow(warnings)]
pub mod agents;
pub mod tools;
pub mod config;
pub mod api;
pub mod error;
pub mod types;
pub mod ai;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Re-export commonly used types
pub use types::{Agent, AgentConfig, Message, Tool, State};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
