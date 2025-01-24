use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::GitAssistantAgent,
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
    }

    Ok(())
}
