use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::{self, AgentRegistry, TransferService, GitAssistantAgent, HaikuAgent, GreeterAgent},
    types::{AgentConfig, Message, Agent, TodoProcessor, TodoTask, TaskPriority, TaskStatus},
    error::Error,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io::Write;
use anyhow::{Result, anyhow};
use chrono::Utc;
use uuid::Uuid;

#[cfg(feature = "project-agent")]
use swarmonomicon::agents::project::ProjectAgent;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional command to execute directly (bypassing greeter)
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Git operations with intelligent commit messages
    Git {
        /// Commit message (if not provided, one will be generated)
        #[arg(short = 'm', long)]
        message: Option<String>,

        /// Create and switch to a new branch
        #[arg(short = 'b', long)]
        branch: Option<String>,

        /// Merge current branch into target branch
        #[arg(short = 't', long)]
        merge: Option<String>,
    },

    /// Initialize a new project
    Init {
        /// Project type (python, rust, or common)
        #[arg(short = 't', long)]
        project_type: String,

        /// Project name
        #[arg(short = 'n', long)]
        name: String,

        /// Project description
        #[arg(short = 'd', long)]
        description: String,
    },

    /// Send a message to the current agent
    Message {
        /// The message to send
        message: String,
    },
}

async fn initialize_registry() -> Result<AgentRegistry> {
    let mut reg = agents::AgentRegistry::new();
    
    let git_assistant = GitAssistantAgent::new(AgentConfig {
        name: "git".to_string(),
        public_description: "Git operations assistant".to_string(),
        instructions: "Help with git operations".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    });

    let haiku = HaikuAgent::new(AgentConfig {
        name: "haiku".to_string(),
        public_description: "Haiku generator".to_string(),
        instructions: "Generate haikus".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    });

    let greeter = GreeterAgent::new(AgentConfig {
        name: "greeter".to_string(),
        public_description: "Greeting agent".to_string(),
        instructions: "Greet users".to_string(),
        tools: vec![],
        downstream_agents: vec![],
        personality: None,
        state_machine: None,
    });

    reg.register("git".to_string(), Box::new(git_assistant)).await
        .map_err(|e| anyhow!("Failed to register git agent: {}", e))?;
    reg.register("haiku".to_string(), Box::new(haiku)).await
        .map_err(|e| anyhow!("Failed to register haiku agent: {}", e))?;
    reg.register("greeter".to_string(), Box::new(greeter)).await
        .map_err(|e| anyhow!("Failed to register greeter agent: {}", e))?;

    Ok(reg)
}

async fn handle_git_command(
    reg: &mut AgentRegistry,
    git_message: String,
    branch_name: String,
    target_branch: String,
) -> Result<()> {
    reg.set_current_agent("git".to_string());
    let input = format!(
        "Message: {}\nBranch: {}\nTarget: {}",
        git_message, branch_name, target_branch
    );
    let task = TodoTask {
        id: Uuid::new_v4().to_string(),
        description: input,
        priority: TaskPriority::Medium,
        source_agent: Some("swarm".to_string()),
        target_agent: "git".to_string(),
        status: TaskStatus::Pending,
        created_at: Utc::now().timestamp(),
        completed_at: None,
    };
    let agent = reg.get("git").ok_or_else(|| anyhow!("Git agent not found"))?;
    agent.process_task(task).await.map_err(|e| anyhow!(e))?;
    Ok(())
}

async fn handle_init_command(
    reg: &mut AgentRegistry,
    init_message: String,
) -> Result<()> {
    reg.set_current_agent("greeter".to_string());
    let task = TodoTask {
        id: Uuid::new_v4().to_string(),
        description: init_message,
        priority: TaskPriority::Medium,
        source_agent: Some("swarm".to_string()),
        target_agent: "greeter".to_string(),
        status: TaskStatus::Pending,
        created_at: Utc::now().timestamp(),
        completed_at: None,
    };
    let agent = reg.get("greeter").ok_or_else(|| anyhow!("Greeter agent not found"))?;
    agent.process_task(task).await.map_err(|e| anyhow!(e))?;
    Ok(())
}

async fn handle_message(
    reg: &mut AgentRegistry,
    message: String,
) -> Result<()> {
    let current_agent_name = reg.get_current_agent().ok_or_else(|| anyhow!("No current agent set"))?;
    let agent = reg.get(&current_agent_name).ok_or_else(|| anyhow!("Current agent not found"))?;
    let task = TodoTask {
        id: Uuid::new_v4().to_string(),
        description: message,
        priority: TaskPriority::Medium,
        source_agent: Some("swarm".to_string()),
        target_agent: current_agent_name.to_string(),
        status: TaskStatus::Pending,
        created_at: Utc::now().timestamp(),
        completed_at: None,
    };
    agent.process_task(task).await.map_err(|e| anyhow!(e))?;
    Ok(())
}

async fn interactive_mode(reg: &mut AgentRegistry) -> Result<()> {
    println!("Enter your message (or 'quit' to exit):");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    while input.trim() != "quit" {
        handle_message(reg, input.trim().to_string()).await?;
        
        println!("Enter your message (or 'quit' to exit):");
        input.clear();
        std::io::stdin().read_line(&mut input)?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut reg = initialize_registry().await?;

    if let Some(command) = cli.command {
        match command {
            Commands::Git { message, branch, merge } => {
                let git_message = message.unwrap_or_else(|| "".to_string());
                let branch_name = branch.unwrap_or_else(|| "".to_string());
                let target_branch = merge.unwrap_or_else(|| "".to_string());
                handle_git_command(&mut reg, git_message, branch_name, target_branch).await?;
            }
            Commands::Init { project_type, name, description } => {
                let init_message = format!("Create {} project '{}' with description: {}", 
                    project_type, name, description);
                handle_init_command(&mut reg, init_message).await?;
            }
            Commands::Message { message } => {
                handle_message(&mut reg, message).await?;
            }
        }
    } else {
        interactive_mode(&mut reg).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    #[cfg(all(feature = "haiku-agent", feature = "git-agent"))]
    async fn test_haiku_git_integration() -> Result<(), Error> {
        let temp_dir = tempdir()?;
        let repo_path = temp_dir.path().join("repo");
        std::fs::create_dir(&repo_path)?;

        // Initialize the registry with test agents
        let registry = Arc::new(RwLock::new(agents::AgentRegistry::new()));
        {
            let mut registry = registry.write().await;

            // Create git agent with temp directory
            #[cfg(feature = "git-agent")]
            let git_assistant = GitAssistantAgent::new(AgentConfig {
                name: "git".to_string(),
                public_description: "Git test agent".to_string(),
                instructions: "Test git operations".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });

            #[cfg(feature = "git-agent")]
            {
                let mut git = git_assistant;
                git.update_working_dir(temp_dir.path().to_path_buf())?;
                registry.register("git".to_string(), Box::new(git)).await?;
            }

            let haiku_agent = HaikuAgent::new(AgentConfig {
                name: "haiku".to_string(),
                public_description: "Test haiku agent".to_string(),
                instructions: "Test haiku generation".to_string(),
                tools: vec![],
                downstream_agents: vec!["git".to_string()],
                personality: None,
                state_machine: None,
            });

            #[cfg(feature = "project-agent")]
            let project_agent = ProjectAgent::new(AgentConfig {
                name: "project".to_string(),
                public_description: "Test project agent".to_string(),
                instructions: "Test project initialization".to_string(),
                tools: vec![],
                downstream_agents: vec!["git".to_string()],
                personality: None,
                state_machine: None,
            }).await?;

            registry.register("haiku".to_string(), Box::new(haiku_agent)).await?;
            #[cfg(feature = "project-agent")]
            registry.register("project".to_string(), Box::new(project_agent)).await?;
        }

        // Create transfer service
        let mut service = TransferService::new(registry.clone());

        // Test haiku generation and git commit
        service.set_current_agent("haiku".to_string());
        let response = service.process_message(Message::new("generate haiku about coding".to_string())).await?;

        assert!(response.content.contains("Generated haiku:"));

        // Verify git commit
        let git_status = std::process::Command::new("git")
            .current_dir(temp_dir.path())
            .args(["log", "--oneline"])
            .output()?;

        let git_log = String::from_utf8_lossy(&git_status.stdout);
        assert!(git_log.contains("[haiku]"));

        // Test project initialization
        service.set_current_agent("project".to_string());
        let response = service.process_message(Message::new("create rust test-project 'A test project'".to_string())).await?;

        assert!(response.content.contains("Project created"));

        // Verify project files were committed
        let git_status = std::process::Command::new("git")
            .current_dir(temp_dir.path())
            .args(["log", "--oneline"])
            .output()?;

        let git_log = String::from_utf8_lossy(&git_status.stdout);
        assert!(git_log.contains("[project]"));

        Ok(())
    }
}
