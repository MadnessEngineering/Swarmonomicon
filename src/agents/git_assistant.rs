use std::process::Command;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, Tool, ToolCall, State};
use crate::tools::ToolRegistry;
use crate::Result;

pub struct GitAssistantAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    current_state: Option<String>,
    working_dir: PathBuf,
}

impl GitAssistantAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tools: ToolRegistry::create_default_tools(),
            current_state: None,
            working_dir: PathBuf::from("."),
        }
    }

    pub fn set_working_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(format!("Directory does not exist: {}", path.display()).into());
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", path.display()).into());
        }
        self.working_dir = path.to_path_buf();
        Ok(())
    }

    fn get_git_diff(&self) -> Result<String> {
        // Check staged changes
        let staged = Command::new("git")
            .current_dir(&self.working_dir)
            .args(["diff", "--staged"])
            .output()?;

        if !staged.stdout.is_empty() {
            return Ok(String::from_utf8_lossy(&staged.stdout).to_string());
        }

        // Check unstaged changes
        let unstaged = Command::new("git")
            .current_dir(&self.working_dir)
            .args(["diff"])
            .output()?;

        let diff = String::from_utf8_lossy(&unstaged.stdout).to_string();
        if diff.is_empty() {
            return Err(format!("No changes detected in directory: {}", self.working_dir.display()).into());
        }

        Ok(diff)
    }

    fn create_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["checkout", "-b", branch_name])
            .output()?;
        Ok(())
    }

    fn stage_changes(&self) -> Result<()> {
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["add", "."])
            .output()?;
        Ok(())
    }

    fn commit_changes(&self, message: &str) -> Result<()> {
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["commit", "-m", message])
            .output()?;
        Ok(())
    }

    fn merge_branch(&self, target_branch: &str) -> Result<()> {
        // Get current branch
        let current = Command::new("git")
            .current_dir(&self.working_dir)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();

        // Switch to target branch
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["checkout", target_branch])
            .output()?;

        // Merge the feature branch
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["merge", &current_branch])
            .output()?;

        Ok(())
    }

    async fn generate_commit_message(&self, diff: &str) -> Result<String> {
        // Check if diff is too large (>4000 chars is a reasonable limit for context)
        const MAX_DIFF_SIZE: usize = 4000;
        if diff.len() > MAX_DIFF_SIZE {
            return Ok(format!(
                "feat: large update ({} files changed)\n\nLarge changeset detected. Please review the changes and provide a manual commit message.",
                diff.matches("diff --git").count()
            ));
        }

        // Use OpenAI to generate commit message
        let openai = reqwest::Client::new();
        let response = openai
            .post("http://127.0.0.1:1234/v1/chat/completions")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "qwen2.5-7b-instruct",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a helpful assistant that generates clear and concise git commit messages. You analyze git diffs and create conventional commit messages that follow best practices. Focus on describing WHAT changed and WHY, being specific but concise. Use the conventional commits format: type(scope): Detailed description\n\nTypes: feat, fix, docs, style, refactor, test, chore\nExample: feat(auth): add password reset functionality"
                    },
                    {
                        "role": "user",
                        "content": format!("Generate a commit message for these changes. If you can't determine the changes clearly, respond with 'NEED_MORE_CONTEXT':\n\n{}", diff)
                    }
                ]
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let message = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("NEED_MORE_CONTEXT")
            .to_string();

        if message == "NEED_MORE_CONTEXT" {
            Ok("Please provide a commit message. The changes are too complex for automatic generation.".to_string())
        } else {
            Ok(message)
        }
    }
}

#[async_trait::async_trait]
impl Agent for GitAssistantAgent {
    fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process_message(&mut self, message: &str) -> Result<Message> {
        let parts: Vec<&str> = message.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(Message {
                content: "Please provide a Git command or changes to commit. Use 'cd <path>' to change working directory.".to_string(),
                role: "assistant".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                metadata: None,
            });
        }

        let mut tool_calls = Vec::new();
        let result = match parts[0] {
            "cd" if parts.len() > 1 => {
                let path = parts[1..].join(" ");
                match self.set_working_dir(path) {
                    Ok(_) => format!("Working directory changed to: {}", self.working_dir.display()),
                    Err(e) => format!("Error changing directory: {}", e),
                }
            },
            "commit" => {
                // Get diff
                let mut params = HashMap::new();
                params.insert("command".to_string(), "diff".to_string());
                params.insert("path".to_string(), self.working_dir.to_string_lossy().to_string());
                let diff = self.tools.execute(&Tool {
                    name: "git".to_string(),
                    description: "Git operations".to_string(),
                    parameters: HashMap::new(),
                }, params).await?;

                if diff.is_empty() {
                    "No changes detected to commit.".to_string()
                } else {
                    // Stage changes
                    let mut params = HashMap::new();
                    params.insert("command".to_string(), "stage".to_string());
                    params.insert("path".to_string(), self.working_dir.to_string_lossy().to_string());
                    self.tools.execute(&Tool {
                        name: "git".to_string(),
                        description: "Git operations".to_string(),
                        parameters: HashMap::new(),
                    }, params).await?;

                    // Generate or use provided commit message
                    let commit_msg = if parts.len() > 1 {
                        parts[1..].join(" ")
                    } else {
                        self.generate_commit_message(&diff).await?
                    };

                    // Commit changes
                    let mut params = HashMap::new();
                    params.insert("command".to_string(), "commit".to_string());
                    params.insert("message".to_string(), commit_msg.clone());
                    params.insert("path".to_string(), self.working_dir.to_string_lossy().to_string());

                    let tool_call = ToolCall {
                        tool: "git".to_string(),
                        parameters: params.clone(),
                        result: None,
                    };
                    tool_calls.push(tool_call);

                    self.tools.execute(&Tool {
                        name: "git".to_string(),
                        description: "Git operations".to_string(),
                        parameters: HashMap::new(),
                    }, params).await?;

                    format!("Changes committed with message: {}", commit_msg)
                }
            },
            "branch" if parts.len() > 1 => {
                let mut params = HashMap::new();
                params.insert("command".to_string(), "branch".to_string());
                params.insert("name".to_string(), parts[1].to_string());
                params.insert("path".to_string(), self.working_dir.to_string_lossy().to_string());

                let tool_call = ToolCall {
                    tool: "git".to_string(),
                    parameters: params.clone(),
                    result: None,
                };
                tool_calls.push(tool_call);

                self.tools.execute(&Tool {
                    name: "git".to_string(),
                    description: "Git operations".to_string(),
                    parameters: HashMap::new(),
                }, params).await?
            },
            "merge" if parts.len() > 1 => {
                let mut params = HashMap::new();
                params.insert("command".to_string(), "merge".to_string());
                params.insert("target".to_string(), parts[1].to_string());
                params.insert("path".to_string(), self.working_dir.to_string_lossy().to_string());

                let tool_call = ToolCall {
                    tool: "git".to_string(),
                    parameters: params.clone(),
                    result: None,
                };
                tool_calls.push(tool_call);

                self.tools.execute(&Tool {
                    name: "git".to_string(),
                    description: "Git operations".to_string(),
                    parameters: HashMap::new(),
                }, params).await?
            },
            _ => "Available commands: cd <path>, commit [message], branch <name>, merge <target>".to_string(),
        };

        Ok(Message {
            content: result,
            role: "assistant".to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            metadata: if tool_calls.is_empty() {
                None
            } else {
                Some(MessageMetadata {
                    tool_calls: Some(tool_calls),
                    state: self.current_state.clone(),
                    confidence: None,
                })
            },
        })
    }

    async fn transfer_to(&mut self, _agent_name: &str) -> Result<()> {
        Ok(())
    }

    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.tools.execute(tool, params).await
    }

    fn get_current_state(&self) -> Option<&State> {
        None
    }
}
