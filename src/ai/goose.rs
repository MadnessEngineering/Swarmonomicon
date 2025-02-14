use std::collections::HashMap;
use serde_json::Value;
use crate::types::Result;
use tokio::process::Command as TokioCommand;
use anyhow::anyhow;
use super::AiProvider;

pub struct GooseClient {
    model: String,
}

impl Default for GooseClient {
    fn default() -> Self {
        Self {
            model: "qwen2.5-7b-instruct".to_string(),
        }
    }
}

impl GooseClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait::async_trait]
impl AiProvider for GooseClient {
    async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String> {
        // Format the messages into a single prompt
        let mut prompt = format!("System: {}\n\n", system_prompt);
        for message in messages {
            if let Some(role) = message.get("role") {
                if let Some(content) = message.get("content") {
                    prompt.push_str(&format!("{}: {}\n", role, content));
                }
            }
        }

        // Execute goose CLI command
        let output = TokioCommand::new("goose")
            .args([
                "run",
                "--text",
                "--model", &self.model,
                &prompt,
            ])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute goose command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| anyhow!("Failed to parse goose output: {}", e))?)
        } else {
            Err(anyhow!("Goose command failed: {}", String::from_utf8_lossy(&output.stderr)).into())
        }
    }
} 
