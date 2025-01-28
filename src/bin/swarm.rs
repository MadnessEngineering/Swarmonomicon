use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::{self, TransferService, GreeterAgent},
    types::{Agent, AgentConfig},
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        *registry = agents::AgentRegistry::create_default_agents(vec![
            AgentConfig {
                name: "greeter".to_string(),
                public_description: "Swarmonomicon's Guide to Unhinged Front Desk Wizardry".to_string(),
                instructions: "Master of controlled chaos and improvisational engineering".to_string(),
                tools: vec![],
                downstream_agents: vec!["git".to_string(), "project".to_string(), "haiku".to_string()],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "git".to_string(),
                public_description: "Git operations with intelligent commit messages".to_string(),
                instructions: "Handles Git operations including commits, branches, and merges".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "project".to_string(),
                public_description: "Project initialization tool".to_string(),
                instructions: "Creates new projects with proper structure and configuration".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: "haiku".to_string(),
                public_description: "Creates haikus".to_string(),
                instructions: "Create haikus".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
        ])?;
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
                    Ok(response) => println!("{}", response.content),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    }

    Ok(())
}
