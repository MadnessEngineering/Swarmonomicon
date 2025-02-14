use std::collections::HashMap;
use serde_json::Value;
use crate::types::Result;
use super::AiProvider;
use tokio::process::Command as TokioCommand;
use anyhow::anyhow;

const DEFAULT_MODEL: &str = "michaelneale/deepseek-r1-goose";

pub struct LocalAiClient {
    model: String,
}

impl Default for LocalAiClient {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

impl LocalAiClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait::async_trait]
impl AiProvider for LocalAiClient {
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

        // Execute ollama CLI command
        let output = TokioCommand::new("ollama")
            .args([
                "run",
                &self.model,
                &prompt,
            ])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute ollama command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| anyhow!("Failed to parse ollama output: {}", e))?)
        } else {
            Err(anyhow!("Ollama command failed: {}", String::from_utf8_lossy(&output.stderr)).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_ai_client_creation() {
        let client = LocalAiClient::new()
            .with_model("test-model".to_string());

        assert_eq!(client.model, "test-model");
    }
}
