use std::process::Command;
use crate::types::{Agent, AgentConfig, AgentResponse, Message};
use crate::Result;

pub struct GitAssistantAgent {
    config: AgentConfig,
}

impl GitAssistantAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    fn get_git_diff(&self) -> Result<String> {
        // Check staged changes
        let staged = Command::new("git")
            .args(["diff", "--staged"])
            .output()?;

        if !staged.stdout.is_empty() {
            return Ok(String::from_utf8_lossy(&staged.stdout).to_string());
        }

        // Check unstaged changes
        let unstaged = Command::new("git")
            .args(["diff"])
            .output()?;

        Ok(String::from_utf8_lossy(&unstaged.stdout).to_string())
    }

    fn create_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .args(["checkout", "-b", branch_name])
            .output()?;
        Ok(())
    }

    fn stage_changes(&self) -> Result<()> {
        Command::new("git")
            .args(["add", "."])
            .output()?;
        Ok(())
    }

    fn commit_changes(&self, message: &str) -> Result<()> {
        Command::new("git")
            .args(["commit", "-m", message])
            .output()?;
        Ok(())
    }

    fn merge_branch(&self, target_branch: &str) -> Result<()> {
        // Get current branch
        let current = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();

        // Switch to target branch
        Command::new("git")
            .args(["checkout", target_branch])
            .output()?;

        // Merge the feature branch
        Command::new("git")
            .args(["merge", &current_branch])
            .output()?;

        Ok(())
    }

    async fn generate_commit_message(&self, diff: &str) -> Result<String> {
        // Use the agent's LLM to generate a commit message
        let prompt = format!(
            "Generate a clear and concise git commit message for these changes:\n\n{}",
            diff
        );

        let response = self.process_message(&prompt).await?;
        Ok(response.content)
    }
}

#[async_trait::async_trait]
impl Agent for GitAssistantAgent {
    fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        // Parse command from message
        let parts: Vec<&str> = message.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(AgentResponse {
                content: "Please provide a Git command or changes to commit.".to_string(),
                should_transfer: false,
                transfer_to: None,
            });
        }

        match parts[0] {
            "commit" => {
                let diff = self.get_git_diff()?;
                if diff.is_empty() {
                    return Ok(AgentResponse {
                        content: "No changes detected to commit.".to_string(),
                        should_transfer: false,
                        transfer_to: None,
                    });
                }

                self.stage_changes()?;

                // Generate or use provided commit message
                let commit_msg = if parts.len() > 1 {
                    parts[1..].join(" ")
                } else {
                    self.generate_commit_message(&diff).await?
                };

                self.commit_changes(&commit_msg)?;

                Ok(AgentResponse {
                    content: format!("Changes committed with message: {}", commit_msg),
                    should_transfer: false,
                    transfer_to: None,
                })
            },
            "branch" if parts.len() > 1 => {
                self.create_branch(parts[1])?;
                Ok(AgentResponse {
                    content: format!("Created and switched to branch: {}", parts[1]),
                    should_transfer: false,
                    transfer_to: None,
                })
            },
            "merge" if parts.len() > 1 => {
                self.merge_branch(parts[1])?;
                Ok(AgentResponse {
                    content: format!("Merged current branch into: {}", parts[1]),
                    should_transfer: false,
                    transfer_to: None,
                })
            },
            _ => Ok(AgentResponse {
                content: "Available commands: commit [message], branch <name>, merge <target>".to_string(),
                should_transfer: false,
                transfer_to: None,
            }),
        }
    }
}
