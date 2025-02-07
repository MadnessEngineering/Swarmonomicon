use std::collections::HashMap;
use std::process::Command;
use async_trait::async_trait;
use crate::tools::ToolExecutor;
use anyhow::{Result, anyhow};
use tokio::process::Command as TokioCommand;

pub struct GooseTool;

impl GooseTool {
    pub fn new() -> Self {
        Self
    }

    async fn execute_command(&self, command: &str) -> Result<String> {
        let output = TokioCommand::new("goose")
            .args(["exec", command])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow!("Command failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    async fn edit_file(&self, file_path: &str, edit_instructions: &str) -> Result<String> {
        let output = TokioCommand::new("goose")
            .args(["edit", file_path, "--instructions", edit_instructions])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to edit file: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow!("File edit failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}

#[async_trait]
impl ToolExecutor for GooseTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let action = params.get("action").ok_or_else(|| anyhow!("Missing action parameter"))?;

        match action.as_str() {
            "exec" => {
                let command = params.get("command").ok_or_else(|| anyhow!("Missing command parameter"))?;
                self.execute_command(command).await
            }
            "edit" => {
                let file_path = params.get("file_path").ok_or_else(|| anyhow!("Missing file_path parameter"))?;
                let instructions = params.get("instructions").ok_or_else(|| anyhow!("Missing instructions parameter"))?;
                self.edit_file(file_path, instructions).await
            }
            _ => Err(anyhow!("Unknown goose action. Use 'exec' or 'edit'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_goose_tool() {
        let tool = GooseTool::new();
        
        // Test command execution
        let mut params = HashMap::new();
        params.insert("action".to_string(), "exec".to_string());
        params.insert("command".to_string(), "echo 'test'".to_string());
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());

        // Test file editing
        let mut params = HashMap::new();
        params.insert("action".to_string(), "edit".to_string());
        params.insert("file_path".to_string(), "test.txt".to_string());
        params.insert("instructions".to_string(), "Add a test line".to_string());
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
    }
} 
