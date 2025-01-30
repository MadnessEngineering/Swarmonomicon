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
use crate::ai::AiClient;

pub struct GitAssistantAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    state_manager: AgentStateManager,
    working_dir: Arc<Mutex<Option<PathBuf>>>,
    current_state: Option<State>,
    ai_client: AiClient,
}

impl GitAssistantAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tools: ToolRegistry::create_default_tools(),
            state_manager: AgentStateManager::new(None),
            working_dir: Arc::new(Mutex::new(None)),
            current_state: None,
            ai_client: AiClient::new(),
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
    pub fn update_working_dir(&self, path: PathBuf) -> Result<()> {
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
        let system_prompt = "You are a helpful assistant that generates clear and concise git commit messages. \
            You analyze git diffs and create conventional commit messages that follow best practices. \
            Focus on describing WHAT changed and WHY, being specific but concise. \
            Use the conventional commits format: type(scope): Detailed description\n\n\
            Types: feat, fix, docs, style, refactor, test, chore\n\
            Example: feat(auth): add password reset functionality";

        let messages = vec![HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), format!(
                "Generate a commit message for these changes. If you can't determine the changes clearly, respond with 'NEED_MORE_CONTEXT':\n\n{}",
                diff
            )),
        ])];

        let message = self.ai_client.chat(system_prompt, messages).await?;

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

        let state = self.get_current_state().await.unwrap_or(None)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "archival".to_string());

        Message::new(content)
            .with_metadata(Some(MessageMetadata::new("git_assistant".to_string())
                .with_personality(traits)
                .with_state(state)))
    }

    fn format_git_response(&self, content: String) -> Message {
        let traits = vec!["helpful".to_string(), "technical".to_string()];
        let state = self.current_state.as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "archival".to_string());

        Message::new(content)
            .with_metadata(Some(MessageMetadata::new("git_assistant".to_string())
                .with_personality(traits)
                .with_state(state)))
    }

    fn handle_git_command(&self, command: &str) -> Message {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"");
        let args = if parts.len() > 1 { &parts[1..] } else { &[] };

        let response = match *cmd {
            "help" | "" => format!(
                "🌟 Quantum Version Control Interface - Your Temporal Archive Assistant\n\n\
                Available timeline manipulation commands:\n\
                - init: Initialize a new temporal nexus (git repository)\n\
                - status: Scan quantum state of current timeline\n\
                - add <files>: Preserve artifacts in the temporal archive\n\
                - commit <message>: Create a quantum state marker\n\
                - branch <name>: Initiate a parallel timeline branch\n\
                - checkout <branch>: Shift to an alternate timeline\n\
                - merge <branch>: Converge timelines into unified reality\n\
                - push: Synchronize local quantum states with the temporal nexus\n\
                - pull: Retrieve quantum state updates from the temporal nexus"
            ),
            "status" => {
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["status"])
                    .output() {
                        Ok(output) => {
                            let status = String::from_utf8_lossy(&output.stdout).to_string();
                            if status.is_empty() {
                                "🌌 This dimension appears to lack a temporal nexus. Initialize one with 'init'".to_string()
                            } else {
                                format!("🔮 Quantum State Analysis:\n{}", status)
                            }
                        },
                        Err(_) => "🌌 This dimension appears to lack a temporal nexus. Initialize one with 'init'".to_string(),
                    }
            },
            "add" => {
                let files = args.join(" ");
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["add"])
                    .args(args)
                    .output() {
                        Ok(_) => format!("🌟 Preparing to preserve the following artifacts in the temporal archive: {}", files),
                        Err(_) => "⚠️ Temporal preservation failed. Is this a valid timeline branch?".to_string(),
                    }
            },
            "commit" => {
                let msg = args.join(" ");
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["commit", "-m", if msg.is_empty() { "archival" } else { &msg }])
                    .output() {
                        Ok(output) => format!("✨ Creating quantum state marker: {}\n{}",
                            if msg.is_empty() { "archival" } else { &msg },
                            String::from_utf8_lossy(&output.stdout)),
                        Err(_) => "⚠️ Failed to create quantum state marker. Are there changes to commit?".to_string(),
                    }
            },
            "branch" => {
                let branch_name = args.join(" ");
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["checkout", "-b", &branch_name])
                    .output() {
                        Ok(_) => format!("🌌 Initiating parallel timeline branch: {}", branch_name),
                        Err(_) => "⚠️ Failed to create parallel timeline. Is this a valid temporal nexus?".to_string(),
                    }
            },
            "checkout" => {
                let branch_name = args.join(" ");
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["checkout", &branch_name])
                    .output() {
                        Ok(_) => format!("🌠 Shifting to timeline: {}", branch_name),
                        Err(_) => "⚠️ Timeline shift failed. Does this reality branch exist?".to_string(),
                    }
            },
            "merge" => {
                let branch_name = args.join(" ");
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["merge", &branch_name])
                    .output() {
                        Ok(output) => format!("🌊 Converging timeline {} with current timeline\n{}",
                            branch_name,
                            String::from_utf8_lossy(&output.stdout)),
                        Err(_) => "⚠️ Timeline convergence failed. Are both realities compatible?".to_string(),
                    }
            },
            "push" => {
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["push"])
                    .output() {
                        Ok(_) => "🚀 Synchronizing local quantum states with the temporal nexus...".to_string(),
                        Err(_) => "⚠️ Temporal synchronization failed. Is the nexus reachable?".to_string(),
                    }
            },
            "pull" => {
                match Command::new("git")
                    .current_dir(self.get_working_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .args(["pull"])
                    .output() {
                        Ok(_) => "📥 Retrieving quantum state updates from the temporal nexus...".to_string(),
                        Err(_) => "⚠️ Failed to retrieve temporal updates. Is the nexus reachable?".to_string(),
                    }
            },
            _ => format!("❓ Unknown temporal operation: {}. Use 'help' to see available commands.", command),
        };

        self.format_git_response(response)
    }
}

#[async_trait]
impl Agent for GitAssistantAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let command = message.content.trim().to_lowercase();
        Ok(self.handle_git_command(&command))
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        let response = format!("Executing tool: {} with parameters: {:?}", tool.name, params);
        Ok(response)
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(self.current_state.clone())
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
        Message::new(content.to_string())
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

    #[cfg(test)]
    fn create_test_agent() -> GitAssistantAgent {
        GitAssistantAgent::new(AgentConfig {
            name: "git".to_string(),
            public_description: "Git test agent".to_string(),
            instructions: "Test git operations".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        })
    }

    #[tokio::test]
    async fn test_help_message() {
        let agent = create_test_agent();
        let response = agent.process_message(Message::new("help".to_string())).await.unwrap();
        assert!(response.content.contains("Quantum"), "Help message should contain quantum theme");
        assert!(response.content.contains("commands"), "Help message should list commands");
    }

    #[tokio::test]
    async fn test_empty_repo_status() {
        let temp_dir = tempdir().unwrap();
        let mut agent = create_test_agent();
        agent.update_working_dir(temp_dir.path().to_path_buf()).unwrap();

        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        assert!(response.content.contains("temporal nexus"),
            "Should indicate missing temporal nexus (git repo)");
    }

    #[tokio::test]
    async fn test_commit_flow() {
        let temp_dir = tempdir().unwrap();
        let mut agent = create_test_agent();
        agent.update_working_dir(temp_dir.path().to_path_buf()).unwrap();

        // Initialize git repo
        Command::new("git")
            .current_dir(&temp_dir.path())
            .args(["init"])
            .output()
            .unwrap();

        // Check status
        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        assert!(response.content.contains("Quantum State Analysis") || response.content.contains("temporal nexus"),
            "Should show repository status");

        // Create a test file
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        // Check status again
        let response = agent.process_message(Message::new("status".to_string())).await.unwrap();
        assert!(response.content.contains("Untracked") || response.content.contains("untracked"),
            "Should show untracked files");

        // Add and commit
        let add_response = agent.process_message(Message::new("add test.txt".to_string())).await.unwrap();
        assert!(add_response.content.contains("preserve") || add_response.content.contains("artifact"),
            "Should indicate file preservation");

        let commit_response = agent.process_message(Message::new("commit Initial commit".to_string())).await.unwrap();
        assert!(commit_response.content.contains("quantum state marker"),
            "Should indicate quantum state marker creation");
    }

    #[tokio::test]
    async fn test_branch_and_merge() {
        let temp_dir = tempdir().unwrap();
        let mut agent = create_test_agent();
        agent.update_working_dir(temp_dir.path().to_path_buf()).unwrap();

        // Initialize and create initial commit
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

        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        let add_response = agent.process_message(Message::new("add test.txt".to_string())).await.unwrap();
        assert!(add_response.content.contains("preserve") || add_response.content.contains("artifact"),
            "Should indicate file preservation");

        let commit_response = agent.process_message(Message::new("commit Initial commit".to_string())).await.unwrap();
        assert!(commit_response.content.contains("quantum state marker"),
            "Should indicate quantum state marker creation");

        // Create and switch to new branch
        let branch_response = agent.process_message(Message::new("branch feature".to_string())).await.unwrap();
        assert!(branch_response.content.contains("parallel timeline") || branch_response.content.contains("timeline branch"),
            "Should indicate parallel timeline creation");

        // Make changes in feature branch
        std::fs::write(temp_dir.path().join("feature.txt"), "feature content").unwrap();
        let add_response = agent.process_message(Message::new("add feature.txt".to_string())).await.unwrap();
        assert!(add_response.content.contains("preserve") || add_response.content.contains("artifact"),
            "Should indicate file preservation");

        let commit_response = agent.process_message(Message::new("commit Feature commit".to_string())).await.unwrap();
        assert!(commit_response.content.contains("quantum state marker"),
            "Should indicate quantum state marker creation");

        // Switch back to main and merge
        let checkout_response = agent.process_message(Message::new("checkout main".to_string())).await.unwrap();
        assert!(checkout_response.content.contains("Shifting to timeline"),
            "Should indicate timeline shift");

        let merge_response = agent.process_message(Message::new("merge feature".to_string())).await.unwrap();
        assert!(merge_response.content.contains("Converging timeline"),
            "Should indicate timeline convergence");
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let (mut agent, _temp_dir) = setup_test_repo().await;
        let response = agent.process_message(Message::new("invalid-command".to_string())).await.unwrap();
        assert!(response.content.contains("Unknown temporal operation"));
    }

    #[tokio::test]
    async fn test_git_commands() {
        let config = AgentConfig {
            name: "git".to_string(),
            public_description: "Git assistant".to_string(),
            instructions: "Help with git commands".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };
        let agent = GitAssistantAgent::new(config);

        let response = agent.process_message(Message::new("add test.txt".to_string())).await.unwrap();
        assert!(response.content.contains("preserve") || response.content.contains("artifact"),
            "Should indicate file preservation");

        let response = agent.process_message(Message::new("commit test commit".to_string())).await.unwrap();
        assert!(response.content.contains("quantum state marker"),
            "Should indicate quantum state marker creation");

        let response = agent.process_message(Message::new("checkout main".to_string())).await.unwrap();
        assert!(response.content.contains("Shifting to timeline"),
            "Should indicate timeline shift");
    }
}
