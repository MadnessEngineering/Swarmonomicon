use std::collections::HashMap;
use std::process::Command;
use async_trait::async_trait;
use crate::tools::ToolExecutor;
use anyhow::{Result, anyhow};
use tokio::process::Command as TokioCommand;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

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
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_goose_tool() -> Result<()> {
        let tool = GooseTool::new();
        let temp_dir = tempdir()?;
        let test_file_path = temp_dir.path().join("test.txt");
        
        // Test 1: Safe command execution
        let mut params = HashMap::new();
        params.insert("action".to_string(), "exec".to_string());
        params.insert("command".to_string(), "echo 'test' > test_output.txt".to_string());
        
        let result = tool.execute(params).await?;
        assert!(result.contains("Successfully executed command"));
        
        // Verify command output
        let output_path = temp_dir.path().join("test_output.txt");
        assert!(output_path.exists(), "Command output file should exist");
        let content = fs::read_to_string(output_path)?;
        assert_eq!(content.trim(), "test");

        // Test 2: File modification with AI assistance
        let mut file = File::create(&test_file_path)?;
        writeln!(file, "function add(a, b) {{\n    return a + b;\n}}")?;

        let mut params = HashMap::new();
        params.insert("action".to_string(), "edit".to_string());
        params.insert("file_path".to_string(), test_file_path.to_str().unwrap().to_string());
        params.insert("instructions".to_string(), "Add input validation".to_string());
        
        let result = tool.execute(params).await?;
        assert!(result.contains("Successfully edited file"));

        // Verify file modifications
        let content = fs::read_to_string(&test_file_path)?;
        assert!(content.contains("typeof"), "Should add type checking");
        assert!(content.contains("isNaN"), "Should add number validation");

        // Test 3: Error handling for invalid commands
        let mut params = HashMap::new();
        params.insert("action".to_string(), "exec".to_string());
        params.insert("command".to_string(), "invalid_command".to_string());
        
        let result = tool.execute(params).await;
        assert!(result.is_err(), "Invalid command should fail");
        assert!(result.unwrap_err().to_string().contains("command not found"));

        // Test 4: Error handling for invalid file paths
        let mut params = HashMap::new();
        params.insert("action".to_string(), "edit".to_string());
        params.insert("file_path".to_string(), "nonexistent.txt".to_string());
        params.insert("instructions".to_string(), "Add comments".to_string());
        
        let result = tool.execute(params).await;
        assert!(result.is_err(), "Invalid file path should fail");
        assert!(result.unwrap_err().to_string().contains("No such file"));

        // Test 5: Command injection prevention
        let mut params = HashMap::new();
        params.insert("action".to_string(), "exec".to_string());
        params.insert("command".to_string(), "echo 'test' && rm -rf /".to_string());
        
        let result = tool.execute(params).await;
        assert!(result.is_err(), "Dangerous command should be blocked");
        assert!(result.unwrap_err().to_string().contains("potentially dangerous"));

        Ok(())
    }
} 
