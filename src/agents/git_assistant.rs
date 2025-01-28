use async_trait::async_trait;
use std::process::Command;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, Tool, ToolCall, State, StateMachine, AgentStateManager};
use crate::tools::ToolRegistry;
use crate::Result;
use rand::Rng;
use chrono;

pub struct GitAssistantAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    state_manager: AgentStateManager,
    working_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl GitAssistantAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tools: ToolRegistry::create_default_tools(),
            state_manager: AgentStateManager::new(None),
            working_dir: Arc::new(Mutex::new(None)),
        }
    }

    // Helper to get working directory or return error
    fn get_working_dir(&self) -> Result<PathBuf> {
        self.working_dir.lock()
            .map_err(|e| format!("Lock error: {}", e))?
            .clone()
            .ok_or_else(|| "Working directory not set".into())
    }

    // Change to use interior mutability pattern
    fn update_working_dir(&self, path: PathBuf) -> Result<()> {
        if path.exists() && path.is_dir() {
            let mut wd = self.working_dir.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            *wd = Some(path);
            Ok(())
        } else {
            Err("Invalid working directory path".into())
        }
    }

    fn get_git_diff(&self) -> Result<String> {
        // Check staged changes
        let staged = Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["diff", "--staged"])
            .output()?;

        if !staged.stdout.is_empty() {
            return Ok(String::from_utf8_lossy(&staged.stdout).to_string());
        }

        // Check unstaged changes
        let unstaged = Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["diff"])
            .output()?;

        let diff = String::from_utf8_lossy(&unstaged.stdout).to_string();
        if diff.is_empty() {
            return Err(format!("No changes detected in directory: {}", self.get_working_dir()?.display()).into());
        }

        Ok(diff)
    }

    fn create_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["checkout", "-b", branch_name])
            .output()?;
        Ok(())
    }

    fn stage_changes(&self) -> Result<()> {
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["add", "."])
            .output()?;
        Ok(())
    }

    fn commit_changes(&self, message: &str) -> Result<()> {
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["commit", "-m", message])
            .output()?;
        Ok(())
    }

    fn merge_branch(&self, target_branch: &str) -> Result<()> {
        // Get current branch
        let current = Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();

        // Switch to target branch
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["checkout", target_branch])
            .output()?;

        // Merge the feature branch
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
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
                // "model": "qwen2.5-7b-instruct",
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

    pub async fn commit_for_agent(&mut self, agent_name: &str, message: &str) -> Result<()> {
        // Stage all changes
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["add", "."])
            .output()?;

        // Commit with provided message
        Command::new("git")
            .current_dir(&self.get_working_dir()?)
            .args(["commit", "-m", &format!("[{}] {}", agent_name, message)])
            .output()?;

        Ok(())
    }

    async fn create_response(&self, content: String) -> Message {
        let traits = vec![
            "meticulous".to_string(),
            "time_traveling".to_string(),
            "version_obsessed".to_string(),
            "historically_minded".to_string(),
            "quantum_branching_enthusiast".to_string(),
        ];

        let mut params = HashMap::new();
        params.insert("style".to_string(), "version_control_archivist".to_string());
        params.insert("tone".to_string(), "scholarly_eccentric".to_string());

        let state = self.get_current_state().await.unwrap_or(None)
            .map(|s| s.prompt)
            .unwrap_or_else(|| "archival".to_string());

        let mut msg = Message::new(&content);
        msg.metadata = MessageMetadata::new("git_assistant".to_string())
            .with_personality(traits)
            .with_state(state);
        msg.parameters = params;
        msg.confidence = Some(0.9);
        msg
    }

    async fn format_git_response(&self, content: &str) -> Result<Message> {
        Ok(Message::new(content.to_string()))
    }

    async fn get_help_message(&self) -> Message {
        self.format_git_response(
            "Welcome to the Temporal Version Archives! I can assist with the following quantum operations:\n\
             - 'status': Observe the current timeline divergence\n\
             - 'add <files>': Prepare artifacts for temporal preservation\n\
             - 'commit [message]': Create a permanent quantum state marker\n\
             - 'branch <name>': Split the timeline into a parallel dimension\n\
             - 'checkout <branch>': Travel to an alternate timeline\n\
             - 'merge <branch>': Converge parallel realities\n\
             - 'push': Synchronize with the central timeline nexus\n\
             - 'pull': Retrieve quantum state updates from the nexus\n\
             - 'cd <path>': Relocate to a different archive sector".to_string()
        ).await
    }

    async fn process_git_command(&self, cmd: &str, args: &[&str]) -> Result<std::process::Output> {
        let wd = self.get_working_dir()?;
        Ok(Command::new(cmd)
            .current_dir(&wd)
            .args(args)
            .output()?)
    }
}

#[async_trait]
impl Agent for GitAssistantAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let parts: Vec<&str> = message.content.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(self.get_help_message().await);
        }

        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let result = match parts[0] {
            "cd" if parts.len() > 1 => {
                let path = parts[1..].join(" ");
                match self.update_working_dir(PathBuf::from(path)) {
                    Ok(_) => format!("Temporal observation point relocated to: {}",
                        self.get_working_dir()?.display()),
                    Err(e) => format!("Timeline sector inaccessible: {}", e),
                }
            }
            "status" => {
                let output = self.process_git_command("git", &["status", "--porcelain"]).await?;
                let status = String::from_utf8_lossy(&output.stdout);
                if status.is_empty() {
                    "The timeline is stable. No quantum fluctuations detected.".to_string()
                } else {
                    format!("Temporal anomalies detected:\n{}", status)
                }
            }
            "add" => {
                let files = parts[1..].join(" ");
                let output = self.process_git_command("git", &["add", &files]).await?;
                format!("Artifacts staged for quantum preservation:\n{}", files)
            }
            "commit" => {
                let message = parts[1..].join(" ");
                let diff = self.get_git_diff()?;
                let commit_msg = if message.is_empty() {
                    self.generate_commit_message(&diff).await?
                } else {
                    message
                };
                let output = self.process_git_command("git", &["commit", "-m", &commit_msg]).await?;
                format!("Quantum state marker created: {}", commit_msg)
            }
            "branch" => {
                let branch_name = parts[1..].join(" ");
                let output = self.process_git_command("git", &["checkout", "-b", &branch_name]).await?;
                format!("Branched into alternate timeline: {}", branch_name)
            }
            "checkout" => {
                let branch_name = parts[1..].join(" ");
                let output = self.process_git_command("git", &["checkout", &branch_name]).await?;
                format!("Quantum leaped to timeline: {}", branch_name)
            }
            "merge" => {
                let branch_name = parts[1..].join(" ");
                let output = self.process_git_command("git", &["merge", &branch_name]).await?;
                format!("Converged quantum realities: {} merged into current timeline", branch_name)
            }
            "push" => {
                let output = self.process_git_command("git", &["push"]).await?;
                "Quantum state synchronized with the central timeline nexus".to_string()
            }
            "pull" => {
                let output = self.process_git_command("git", &["pull"]).await?;
                "Quantum state updates retrieved from the central timeline nexus".to_string()
            }
            _ => {
                format!("Unknown temporal operation: {}. Use 'help' for available commands.", parts[0])
            }
        };

        Ok(self.format_git_response(&result).await?)
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        let response = format!("Executing tool: {} with parameters: {:?}", tool.name, params);
        Ok(response)
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(None)
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_message(content: &str) -> Message {
        Message::new(content)
    }

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "git".to_string(),
            public_description: "Git operations assistant".to_string(),
            instructions: "Help with git operations".to_string(),
            tools: Vec::new(),
            downstream_agents: Vec::new(),
            personality: None,
            state_machine: None,
        }
    }

    async fn setup_test_repo() -> (GitAssistantAgent, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let mut agent = GitAssistantAgent::new(create_test_config());
        agent.update_working_dir(temp_dir.path().to_path_buf()).unwrap();

        // Initialize git repo
        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["init"])
            .output()
            .unwrap();

        // Configure git user for commits
        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();

        // Create initial commit to allow branch creation
        fs::write(
            temp_dir.path().join("initial.txt"),
            "Initial commit",
        ).unwrap();

        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["add", "initial.txt"])
            .output()
            .unwrap();

        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        (agent, temp_dir)
    }

    #[tokio::test]
    async fn test_empty_repo_status() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        println!("Status response: {}", response.content);
        assert!(response.content.contains("The timeline is stable"));
    }

    #[tokio::test]
    async fn test_commit_flow() {
        let (mut agent, temp_dir) = setup_test_repo().await;

        // Create a test file
        let test_file_path = temp_dir.path().join("test.txt");
        fs::write(&test_file_path, "Test file contents").unwrap();

        // Check status
        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        assert!(response.content.contains("Untracked files"));
        assert!(response.content.contains("test.txt"));

        // Stage the file
        let response = agent.process_message(Message::new("add test.txt").to_string()).await.unwrap();
        assert!(response.content.contains("Successfully staged"));

        // Check status again
        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        assert!(response.content.contains("Artifacts prepared for temporal preservation"));
        assert!(response.content.contains("test.txt"));

        // Commit
        let response = agent.process_message(Message::new("commit test commit").to_string()).await.unwrap();
        assert!(response.content.contains("Successfully committed"));
    }

    #[tokio::test]
    async fn test_branch_and_merge() {
        let (mut agent, temp_dir) = setup_test_repo().await;

        // Create and switch to a new branch
        let response = agent.process_message(Message::new("branch feature-test".to_string())).await.unwrap();
        assert!(response.content.contains("Created and switched to new branch"));

        // Make a change and commit
        let test_file_path = temp_dir.path().join("test2.txt");
        fs::write(&test_file_path, "Test file in branch").unwrap();
        agent.process_message(Message::new("add test2.txt").to_string()).await.unwrap();
        agent.process_message(Message::new("commit branch test").to_string()).await.unwrap();

        // Switch back to main branch
        let response = agent.process_message(Message::new("checkout main").to_string()).await.unwrap();
        assert!(response.content.contains("Switched to branch"));

        // List branches
        let response = agent.process_message(Message::new("branch".to_string())).await.unwrap();
        assert!(response.content.contains("feature-test"));
    }

    #[tokio::test]
    async fn test_help_message() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("".to_string())).await.unwrap();
        assert!(response.content.contains("Welcome to the Temporal Version Archives"));
        assert!(response.content.contains("quantum operations"));
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("invalid-command".to_string())).await.unwrap();
        assert!(response.content.contains("Unknown temporal operation"));
    }
}
