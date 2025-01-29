use reqwest::Client;
use serde_json::Value;
use crate::types::Result;
use std::collections::HashMap;

const DEFAULT_AI_ENDPOINT: &str = "http://127.0.0.1:1234/v1/chat/completions";
const DEFAULT_MODEL: &str = "qwen2.5-7b-instruct";

pub struct AiClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl Default for AiClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            endpoint: DEFAULT_AI_ENDPOINT.to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

impl AiClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String> {
        let mut chat_messages = vec![
            HashMap::from([
                ("role".to_string(), "system".to_string()),
                ("content".to_string(), system_prompt.to_string()),
            ])
        ];
        chat_messages.extend(messages);

        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": chat_messages
            }))
            .send()
            .await?;

        let data: Value = response.json().await?;
        Ok(data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("I apologize, but I'm having trouble processing that request.")
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_client_creation() {
        let client = AiClient::new()
            .with_endpoint("http://test.endpoint".to_string())
            .with_model("test-model".to_string());

        assert_eq!(client.endpoint, "http://test.endpoint");
        assert_eq!(client.model, "test-model");
    }
}
