use std::collections::HashMap;
use std::process::Command;
use async_trait::async_trait;
use crate::tools::ToolExecutor;
use anyhow::{Result, anyhow};

pub struct GitTool;

impl GitTool {
    pub fn new() -> Self {
        Self
    }

    fn get_git_diff(&self) -> Result<String> {
        // Check staged changes
        let staged = Command::new("git")
            .args(["diff", "--staged"])
            .output()
            .map_err(|e| anyhow!("Failed to get staged changes: {}", e))?;

        if !staged.stdout.is_empty() {
            return Ok(String::from_utf8_lossy(&staged.stdout).to_string());
        }

        // Check unstaged changes
        let unstaged = Command::new("git")
            .args(["diff"])
            .output()
            .map_err(|e| anyhow!("Failed to get unstaged changes: {}", e))?;

        Ok(String::from_utf8_lossy(&unstaged.stdout).to_string())
    }

    fn create_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .args(["checkout", "-b", branch_name])
            .output()
            .map_err(|e| anyhow!("Failed to create branch: {}", e))?;
        Ok(())
    }

    fn stage_changes(&self) -> Result<()> {
        Command::new("git")
            .args(["add", "."])
            .output()
            .map_err(|e| anyhow!("Failed to stage changes: {}", e))?;
        Ok(())
    }

    fn commit_changes(&self, message: &str) -> Result<()> {
        Command::new("git")
            .args(["commit", "-m", message])
            .output()
            .map_err(|e| anyhow!("Failed to commit changes: {}", e))?;
        Ok(())
    }

    fn merge_branch(&self, target_branch: &str) -> Result<()> {
        // Get current branch
        let current = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| anyhow!("Failed to get current branch: {}", e))?;
        let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();

        // Switch to target branch
        Command::new("git")
            .args(["checkout", target_branch])
            .output()
            .map_err(|e| anyhow!("Failed to checkout target branch: {}", e))?;

        // Merge the feature branch
        Command::new("git")
            .args(["merge", &current_branch])
            .output()
            .map_err(|e| anyhow!("Failed to merge branch: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for GitTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let command = params.get("command").ok_or_else(|| anyhow!("Missing command parameter"))?;

        match command.as_str() {
            "diff" => {
                let diff = self.get_git_diff()?;
                Ok(diff)
            }
            "branch" => {
                let name = params.get("name").ok_or_else(|| anyhow!("Missing branch name"))?;
                self.create_branch(name)?;
                Ok(format!("Created and switched to branch: {}", name))
            }
            "stage" => {
                self.stage_changes()?;
                Ok("Changes staged successfully".to_string())
            }
            "commit" => {
                let message = params.get("message").ok_or_else(|| anyhow!("Missing commit message"))?;
                self.commit_changes(message)?;
                Ok(format!("Changes committed with message: {}", message))
            }
            "merge" => {
                let target = params.get("target").ok_or_else(|| anyhow!("Missing target branch"))?;
                self.merge_branch(target)?;
                Ok(format!("Merged current branch into: {}", target))
            }
            _ => Err(anyhow!("Unknown git command")),
        }
    }
}
