use std::collections::HashMap;
use serde_json::Value;
use anyhow::{Result, anyhow};
use super::AiProvider;
use tokio::process::Command as TokioCommand;
use tracing::{debug, warn, error};

const DEFAULT_MODEL: &str = "qwen2.5";
const OLLAMA_CMD: &str = "ollama";

#[derive(Debug, Clone)]
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

    async fn check_model_availability(&self) -> Result<bool> {
        debug!("Checking availability of model: {}", self.model);
        let output = TokioCommand::new(OLLAMA_CMD)
            .args(["list"])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute ollama list command: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("Failed to list models: {}", err);
            return Err(anyhow!("Failed to list models: {}", err));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains(&self.model))
    }

    async fn ensure_model(&self) -> Result<()> {
        if !self.check_model_availability().await? {
            debug!("Model {} not found, attempting to pull", self.model);
            let output = TokioCommand::new(OLLAMA_CMD)
                .args(["pull", &self.model])
                .output()
                .await
                .map_err(|e| anyhow!("Failed to pull model: {}", e))?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                error!("Failed to pull model {}: {}", self.model, err);
                return Err(anyhow!("Failed to pull model {}: {}", self.model, err));
            }
        }
        Ok(())
    }

    fn format_prompt(&self, system_prompt: &str, messages: &[HashMap<String, String>]) -> String {
        let mut formatted = String::new();
        
        // Add system prompt with clear separator
        formatted.push_str(&format!("### System:\n{}\n\n", system_prompt));
        
        // Add conversation history with clear role markers
        for message in messages {
            if let (Some(role), Some(content)) = (message.get("role"), message.get("content")) {
                formatted.push_str(&format!("### {}:\n{}\n\n", role, content));
            }
        }

        formatted
    }
}

#[async_trait::async_trait]
impl AiProvider for LocalAiClient {
    async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String> {
        // Ensure model is available
        self.ensure_model().await?;

        // Format the messages into a structured prompt
        let prompt = self.format_prompt(system_prompt, &messages);
        debug!("Sending prompt to Ollama model {}", self.model);

        // Execute ollama CLI command with timeout
        let output = TokioCommand::new(OLLAMA_CMD)
            .args([
                "run",
                &self.model,
                &prompt,
            ])
            .output()
            .await
            .map_err(|e| {
                error!("Failed to execute ollama command: {}", e);
                anyhow!("Failed to execute ollama command: {}", e)
            })?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| {
                    error!("Failed to parse ollama output: {}", e);
                    anyhow!("Failed to parse ollama output: {}", e)
                })
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("Ollama command failed: {}", err);
            Err(anyhow!("Ollama command failed: {}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_ai_client_creation() {
        let client = LocalAiClient::new()
            .with_model("codellama".to_string());

        assert_eq!(client.model, "codellama");
    }

    #[tokio::test]
    async fn test_prompt_formatting() {
        let client = LocalAiClient::new();
        let system_prompt = "You are a helpful assistant";
        let messages = vec![
            HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), "Hello".to_string()),
            ]),
            HashMap::from([
                ("role".to_string(), "assistant".to_string()),
                ("content".to_string(), "Hi there!".to_string()),
            ]),
        ];

        let formatted = client.format_prompt(system_prompt, &messages);
        assert!(formatted.contains("### System:"));
        assert!(formatted.contains("### user:"));
        assert!(formatted.contains("### assistant:"));
    }

    #[tokio::test]
    async fn test_model_availability_check() {
        let client = LocalAiClient::new();
        let result = client.check_model_availability().await;
        assert!(result.is_ok(), "Model availability check should not error");
    }
}
