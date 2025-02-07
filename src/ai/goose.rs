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
        // Convert messages to JSON format expected by Goose
        let mut chat_messages = vec![
            HashMap::from([
                ("role".to_string(), "system".to_string()),
                ("content".to_string(), system_prompt.to_string()),
            ])
        ];
        chat_messages.extend(messages);

        let json_messages = serde_json::to_string(&chat_messages)?;

        // Execute Goose CLI command
        let output = TokioCommand::new("goose")
            .args([
                "chat",
                "--model", &self.model,
                "--messages", &json_messages,
            ])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute Goose command: {}", e))?;

        if output.status.success() {
            let response: Value = serde_json::from_slice(&output.stdout)?;
            Ok(response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("I apologize, but I'm having trouble processing that request.")
                .to_string())
        } else {
            Err(anyhow!("Goose command failed: {}", String::from_utf8_lossy(&output.stderr)).into())
        }
    }
} 
