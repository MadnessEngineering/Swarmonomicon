use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::{self, TransferService, GreeterAgent, ProjectInitAgent},
    types::{Agent, AgentConfig},
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use swarmonomicon::agents::git_assistant::GitAssistantAgent;
use swarmonomicon::agents::haiku::HaikuAgent;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    // Initialize the global registry with default agents
    let registry = agents::GLOBAL_REGISTRY.clone();
    {
        let mut registry = registry.write().await;

        // Create agent instances
        let mut git_assistant = GitAssistantAgent::new(AgentConfig {
            name: "git".to_string(),
            public_description: "Git operations with intelligent commit messages".to_string(),
            instructions: "Handles Git operations including commits, branches, and merges".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });
        git_assistant.set_working_dir("./").unwrap_or_else(|e| eprintln!("Warning: Failed to set git working directory: {}", e));

        let haiku_agent = HaikuAgent::new(AgentConfig {
            name: "haiku".to_string(),
            public_description: "Creates haikus".to_string(),
            instructions: "Create haikus".to_string(),
            tools: vec![],
            downstream_agents: vec!["git".to_string()],
            personality: None,
            state_machine: None,
        });

        let project_agent = ProjectInitAgent::new(AgentConfig {
            name: "project".to_string(),
            public_description: "Project initialization tool".to_string(),
            instructions: "Creates new projects with proper structure and configuration".to_string(),
            tools: vec![],
            downstream_agents: vec!["git".to_string()],
            personality: None,
            state_machine: None,
        });

        let greeter_agent = GreeterAgent::new(AgentConfig {
            name: "greeter".to_string(),
            public_description: "Swarmonomicon's Guide to Unhinged Front Desk Wizardry".to_string(),
            instructions: "Master of controlled chaos and improvisational engineering".to_string(),
            tools: vec![],
            downstream_agents: vec!["git".to_string(), "project".to_string(), "haiku".to_string()],
            personality: None,
            state_machine: None,
        });

        // Register agents
        registry.register(git_assistant)?;
        registry.register(haiku_agent)?;
        registry.register(project_agent)?;
        registry.register(greeter_agent)?;
    }

    // Create transfer service starting with greeter
    let mut service = TransferService::new(registry.clone());
    service.set_current_agent("greeter".to_string());

    match cli.command {
        Some(Commands::Git { message, branch, merge }) => {
            // Transfer to git agent
            service.transfer("greeter", "git").await?;

            // Process command
            let command = if let Some(msg) = message {
                format!("commit {}", msg)
            } else if let Some(branch_name) = branch {
                format!("branch {}", branch_name)
            } else if let Some(target) = merge {
                format!("merge {}", target)
            } else {
                "commit".to_string() // Default to auto-commit
            };

            let response = service.process_message(&command).await?;
            println!("{}", response.content);
        }
        Some(Commands::Init { project_type, name, description }) => {
            // Transfer to project agent
            service.transfer("greeter", "project").await?;

            // Process command
            let command = format!("create {} {} {}", project_type, name, description);
            let response = service.process_message(&command).await?;
            println!("{}", response.content);
        }
        None => {
            // Interactive mode with greeter
            println!("Welcome to Swarmonomicon! Type 'help' for available commands.");

            let mut buffer = String::new();
            loop {
                buffer.clear();
                if std::io::stdin().read_line(&mut buffer).is_err() {
                    break;
                }

                let message = buffer.trim();
                if message.is_empty() {
                    continue;
                }

                if message == "exit" || message == "quit" {
                    break;
                }

                match service.process_message(message).await {
                    Ok(response) => {
                        println!("{}", response.content);

                        // Check for haiku generation and commit if needed
                        if response.content.contains("Generated haiku:") &&
                           service.get_current_agent().as_deref() == Some("haiku") {
                            // Get the git agent and commit the haiku
                            let mut registry = registry.write().await;
                            if let Some(mut git_agent) = registry.get_mut("git") {
                                let haiku = response.content.replace("Generated haiku:\n", "");
                                if let Err(e) = git_agent.commit_for_agent("haiku", &haiku).await {
                                    eprintln!("Failed to commit haiku: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_haiku_git_integration() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Set up a temporary directory for git
        let temp_dir = tempdir()?;

        // Initialize the registry with test agents
        let registry = Arc::new(RwLock::new(agents::AgentRegistry::new()));
        {
            let mut registry = registry.write().await;

            // Create git agent with temp directory
            let mut git_assistant = GitAssistantAgent::new(AgentConfig {
                name: "git".to_string(),
                public_description: "Git test agent".to_string(),
                instructions: "Test git operations".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });
            git_assistant.set_working_dir(temp_dir.path())?;

            let haiku_agent = HaikuAgent::new(AgentConfig {
                name: "haiku".to_string(),
                public_description: "Test haiku agent".to_string(),
                instructions: "Test haiku generation".to_string(),
                tools: vec![],
                downstream_agents: vec!["git".to_string()],
                personality: None,
                state_machine: None,
            });

            let project_agent = ProjectInitAgent::new(AgentConfig {
                name: "project".to_string(),
                public_description: "Test project agent".to_string(),
                instructions: "Test project initialization".to_string(),
                tools: vec![],
                downstream_agents: vec!["git".to_string()],
                personality: None,
                state_machine: None,
            });

            registry.register(git_assistant)?;
            registry.register(haiku_agent)?;
            registry.register(project_agent)?;
        }

        // Create transfer service
        let mut service = TransferService::new(registry.clone());

        // Test haiku generation and git commit
        service.set_current_agent("haiku".to_string());
        let response = service.process_message("generate haiku about coding").await?;

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
        let response = service.process_message("create rust test-project 'A test project'").await?;

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
