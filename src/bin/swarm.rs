use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::{GitAssistantAgent, ProjectInitAgent},
    types::{Agent, AgentConfig},
};
use std::error::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    match cli.command {
        Commands::Git { message, branch, merge } => {
            // Create Git agent config
            let config = AgentConfig {
                name: "git".to_string(),
                public_description: "Git operations with intelligent commit messages".to_string(),
                instructions: "Handles Git operations including commits, branches, and merges".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            };

            // Create Git agent
            let mut agent = GitAssistantAgent::new(config);

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

            let response = agent.process_message(&command).await?;
            println!("{}", response.content);
        }
        Commands::Init { project_type, name, description } => {
            // Create Project Init agent config
            let config = AgentConfig {
                name: "project".to_string(),
                public_description: "Project initialization tool".to_string(),
                instructions: "Creates new projects with proper structure and configuration".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            };

            // Create Project Init agent
            let mut agent = ProjectInitAgent::new(config);

            // Process command
            let command = format!("create {} {} {}", project_type, name, description);
            let response = agent.process_message(&command).await?;
            println!("{}", response.content);
        }
    }

    Ok(())
}
