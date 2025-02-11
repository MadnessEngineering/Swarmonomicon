use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::Tool;
use anyhow::Result;

mod git;
mod project;
mod object_detection;
mod screenshot_detection;
pub mod todo;
mod goose;

#[cfg(feature = "yolo")]
pub mod yolo;

pub use git::GitTool;
pub use project::ProjectTool;
pub use object_detection::ObjectDetectionTool;
pub use screenshot_detection::ScreenshotDetectionTool;
pub use todo::TodoTool;
pub use goose::GooseTool;

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String>;
}

pub struct AgentTransferTool {
    target_agent: String,
}

impl AgentTransferTool {
    pub fn new(target_agent: String) -> Self {
        Self { target_agent }
    }
}

#[async_trait]
impl ToolExecutor for AgentTransferTool {
    async fn execute(&self, _params: HashMap<String, String>) -> Result<String> {
        unimplemented!("Agent transfer tool execution not yet implemented")
    }
}

pub struct OpenAITool {
    model: String,
    api_key: String,
}

impl OpenAITool {
    pub fn new(model: String, api_key: String) -> Self {
        Self { model, api_key }
    }
}

// #[async_trait]
// impl ToolExecutor for OpenAITool {
//     async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
//         unimplemented!("OpenAI tool execution not yet implemented")
//     }
// }

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: ToolExecutor + 'static>(&mut self, name: String, executor: T) {
        self.tools.insert(name, Box::new(executor));
    }

    pub async fn execute(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        if let Some(executor) = self.tools.get(&tool.name) {
            executor.execute(params).await
        } else {
            Err(anyhow::anyhow!("Tool not found in registry"))
        }
    }

    pub async fn create_default_tools() -> Result<Self> {
        let mut registry = Self::new();

        // Register Git tool
        registry.register("git".to_string(), GitTool::new());

        // Register Project tool
        registry.register("project".to_string(), ProjectTool::new());

        // Register Todo tool
        let todo_tool = TodoTool::new().await?;
        registry.register("todo".to_string(), todo_tool);

        // Register Goose tool
        registry.register("goose".to_string(), GooseTool::new());

        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool;

    #[async_trait]
    impl ToolExecutor for MockTool {
        async fn execute(&self, _params: HashMap<String, String>) -> Result<String> {
            Ok("mock result".to_string())
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register("mock".to_string(), MockTool);

        let tool = Tool {
            name: "mock".to_string(),
            description: "A mock tool".to_string(),
            parameters: HashMap::new(),
        };

        let result = registry.execute(&tool, HashMap::new()).await.unwrap();
        assert_eq!(result, "mock result");
    }
}
