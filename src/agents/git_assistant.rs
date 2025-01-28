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
            .current_dir(&self.working_dir)
            .args(["add", "."])
            .output()?;

        // Commit with provided message
        Command::new("git")
            .current_dir(&self.working_dir)
            .args(["commit", "-m", &format!("[{}] {}", agent_name, message)])
            .output()?;

        Ok(())
    }

    async fn format_git_response(&self, content: String) -> Result<Message> {
        let msg = Message::new(content);
        Ok(msg)
    }
}

#[async_trait::async_trait]
impl Agent for GitAssistantAgent {
    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }

    async fn process_message(&mut self, message: Message) -> Result<Message> {
        let parts: Vec<&str> = message.content.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(self.format_git_response("I can help you with Git operations! Try these commands:\n\
                         - status: Show repository status\n\
                         - add <files>: Stage files (use '.' for all)\n\
                         - commit [message]: Commit changes with optional message\n\
                         - branch <name>: Create a new branch\n\
                         - checkout <branch>: Switch branches\n\
                         - merge <branch>: Merge a branch\n\
                         - push: Push changes to remote\n\
                         - pull: Pull changes from remote\n\
                         - cd <path>: Change working directory".to_string()).await?);
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
            "status" => {
                let output = Command::new("git")
                    .current_dir(&self.working_dir)
                    .args(["status", "--porcelain", "-b"])
                    .output()?;

                let status = String::from_utf8_lossy(&output.stdout);
                let mut response = String::new();

                // Parse branch info
                if let Some(branch_line) = status.lines().next() {
                    if branch_line.starts_with("## ") {
                        response.push_str(&format!("On branch: {}\n\n", &branch_line[3..]));
                    }
                }

                // Parse file status
                let mut staged = Vec::new();
                let mut modified = Vec::new();
                let mut untracked = Vec::new();

                for line in status.lines().skip(1) {
                    if line.len() < 3 { continue; }
                    let (status_code, file) = line.split_at(3);
                    match &status_code[..2] {
                        "M " => modified.push(file.trim()),
                        "A " => staged.push(file.trim()),
                        "??" => untracked.push(file.trim()),
                        _ => {}
                    }
                }

                if staged.is_empty() && modified.is_empty() && untracked.is_empty() {
                    response.push_str("Repository is clean. No changes detected.");
                } else {
                    if !staged.is_empty() {
                        response.push_str("Changes staged for commit:\n");
                        for file in staged {
                            response.push_str(&format!("  - {}\n", file));
                        }
                        response.push('\n');
                    }

                    if !modified.is_empty() {
                        response.push_str("Modified files:\n");
                        for file in modified {
                            response.push_str(&format!("  - {}\n", file));
                        }
                        response.push('\n');
                    }

                    if !untracked.is_empty() {
                        response.push_str("Untracked files:\n");
                        for file in untracked {
                            response.push_str(&format!("  - {}\n", file));
                        }
                    }
                }

                response
            },
            "add" => {
                if parts.len() < 2 {
                    "Please specify files to stage (use '.' for all files)".to_string()
                } else {
                    let files = &parts[1..];
                    let mut success = true;
                    let mut errors = Vec::new();

                    for file in files {
                        let output = Command::new("git")
                            .current_dir(&self.working_dir)
                            .args(["add", file])
                            .output()?;

                        if !output.status.success() {
                            success = false;
                            errors.push(format!("Failed to stage '{}': {}",
                                file,
                                String::from_utf8_lossy(&output.stderr)));
                        }
                    }

                    if success {
                        format!("Successfully staged: {}", files.join(", "))
                    } else {
                        format!("Errors occurred while staging:\n{}", errors.join("\n"))
                    }
                }
            },
            "commit" => {
                // Get diff for staged changes
                let output = Command::new("git")
                    .current_dir(&self.working_dir)
                    .args(["diff", "--staged"])
                    .output()?;

                let diff = String::from_utf8_lossy(&output.stdout).to_string();
                if diff.is_empty() {
                    "No staged changes to commit. Use 'git add' to stage files first.".to_string()
                } else {
                    // Use provided message or generate one
                    let commit_msg = if parts.len() > 1 {
                        parts[1..].join(" ")
                    } else {
                        self.generate_commit_message(&diff).await?
                    };

                    // Commit changes
                    let output = Command::new("git")
                        .current_dir(&self.working_dir)
                        .args(["commit", "-m", &commit_msg])
                        .output()?;

                    if output.status.success() {
                        format!("Successfully committed changes with message:\n{}", commit_msg)
                    } else {
                        format!("Failed to commit: {}", String::from_utf8_lossy(&output.stderr))
                    }
                }
            },
            "branch" => {
                if parts.len() < 2 {
                    // List branches if no name provided
                    let output = Command::new("git")
                        .current_dir(&self.working_dir)
                        .args(["branch", "--list"])
                        .output()?;

                    let branches = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|line| if line.starts_with('*') {
                            format!("{} (current)", line)
                        } else {
                            line.to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!("Available branches:\n{}", branches)
                } else {
                    // Create new branch
                    match self.create_branch(&parts[1]) {
                        Ok(_) => format!("Created and switched to new branch: {}", parts[1]),
                        Err(e) => format!("Failed to create branch: {}", e),
                    }
                }
            },
            "checkout" if parts.len() > 1 => {
                let output = Command::new("git")
                    .current_dir(&self.working_dir)
                    .args(["checkout", parts[1]])
                    .output()?;

                if output.status.success() {
                    format!("Switched to branch: {}", parts[1])
                } else {
                    format!("Failed to switch branches: {}", String::from_utf8_lossy(&output.stderr))
                }
            },
            "merge" if parts.len() > 1 => {
                match self.merge_branch(parts[1]) {
                    Ok(_) => format!("Successfully merged branch: {}", parts[1]),
                    Err(e) => format!("Failed to merge branch: {}", e),
                }
            },
            "push" => {
                let output = Command::new("git")
                    .current_dir(&self.working_dir)
                    .args(["push"])
                    .output()?;

                if output.status.success() {
                    "Successfully pushed changes to remote".to_string()
                } else {
                    format!("Failed to push changes: {}", String::from_utf8_lossy(&output.stderr))
                }
            },
            "pull" => {
                let output = Command::new("git")
                    .current_dir(&self.working_dir)
                    .args(["pull"])
                    .output()?;

                if output.status.success() {
                    "Successfully pulled changes from remote".to_string()
                } else {
                    format!("Failed to pull changes: {}", String::from_utf8_lossy(&output.stderr))
                }
            },
            _ => format!("Unknown command: {}. Type 'help' for available commands.", parts[0]),
        };

        Ok(self.format_git_response(result).await?)
    }

    async fn transfer_to(&mut self, _target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.tools.execute(tool, params).await
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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
        agent.set_working_dir(&temp_dir.path()).unwrap();

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
        let response = agent.process_message(Message::new("status")).await.unwrap();
        println!("Status response: {}", response.content);
        // After initial commit, repo should be clean
        assert!(response.content.contains("Repository is clean"));
    }

    #[tokio::test]
    async fn test_add_and_commit() {
        let (mut agent, temp_dir) = setup_test_repo().await;

        // Create a test file
        fs::write(
            temp_dir.path().join("test.txt"),
            "Hello, World!",
        ).unwrap();

        // Check status
        let response = agent.process_message(Message::new("status")).await.unwrap();
        assert!(response.content.contains("Untracked files"));
        assert!(response.content.contains("test.txt"));

        // Stage the file
        let response = agent.process_message(Message::new("add test.txt")).await.unwrap();
        assert!(response.content.contains("Successfully staged"));

        // Commit the file
        let response = agent.process_message(Message::new("commit Initial commit")).await.unwrap();
        assert!(response.content.contains("Successfully committed changes"));
    }

    #[tokio::test]
    async fn test_branch_operations() {
        let (mut agent, _temp_dir) = setup_test_repo().await;

        // Create and switch to a new branch
        let response = agent.process_message(Message::new("branch feature-test")).await.unwrap();
        assert!(response.content.contains("Created and switched to new branch"));

        // List branches (should show both main/master and feature-test)
        let response = agent.process_message(Message::new("branch")).await.unwrap();
        assert!(response.content.contains("feature-test"));
    }

    #[tokio::test]
    async fn test_help_message() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("")).await.unwrap();
        assert!(response.content.contains("status:"));
        assert!(response.content.contains("add"));
        assert!(response.content.contains("commit"));
        assert!(response.content.contains("branch"));
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("invalid-command")).await.unwrap();
        assert!(response.content.contains("Unknown command"));
        assert!(response.content.contains("Type 'help'"));
    }
}
