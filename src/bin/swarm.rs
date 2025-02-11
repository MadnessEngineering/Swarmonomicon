use clap::{Parser, Subcommand};
use swarmonomicon::{
    agents::{self, TransferService, GreeterAgent},
    types::{Agent, AgentConfig, Message},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io::Write;
use swarmonomicon::error::Error;

#[cfg(feature = "git-agent")]
use swarmonomicon::agents::git_assistant::GitAssistantAgent;

#[cfg(feature = "haiku-agent")]
use swarmonomicon::agents::haiku::HaikuAgent;

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
}

async fn initialize_registry() -> Result<(), Error> {
    let registry = agents::GLOBAL_REGISTRY.clone();
    {
        let mut reg = registry.write().await;

        #[cfg(feature = "git-agent")]
        {
            let git_assistant = GitAssistantAgent::new(AgentConfig {
                name: "git".to_string(),
                public_description: "Git operations with intelligent commit messages".to_string(),
                instructions: "Manages git operations with quantum-themed messages".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });
            reg.register("git".to_string(), Box::new(git_assistant)).await?;
        }
        #[cfg(feature = "haiku-agent")]
        {
            let haiku = HaikuAgent::new(AgentConfig {
                name: "haiku".to_string(),
                public_description: "Creates haikus from user input".to_string(),
                instructions: "Creates haikus based on user input and context".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            });
            reg.register("haiku".to_string(), Box::new(haiku)).await?;
        }
        #[cfg(feature = "project-agent")]
        {
            let project_agent = ProjectAgent::new(AgentConfig {
                name: "project".to_string(),
                public_description: "Project initialization tool".to_string(),
                instructions: "Creates new projects with proper structure and configuration".to_string(),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            }).await?;
            reg.register("project".to_string(), Box::new(project_agent)).await?;
        }

        // Register greeter agent (always available)
        let greeter = GreeterAgent::new(AgentConfig {
            name: "greeter".to_string(),
            public_description: "Quantum Greeter".to_string(),
            instructions: "Master of controlled chaos and improvisational engineering".to_string(),
            tools: vec![],
            downstream_agents: vec!["git".to_string(), "project".to_string(), "haiku".to_string()],
            personality: None,
            state_machine: None,
        });
        reg.register("greeter".to_string(), Box::new(greeter)).await?;
    }
    Ok(())
}

async fn handle_git_command(service: &mut TransferService, message: Option<String>, branch: Option<String>, merge: Option<String>) -> Result<(), Error> {
    service.set_current_agent("git".to_string());

    let git_message = match message {
        Some(msg) => msg,
        None => "".to_string(),
    };

    let response = service.process_message(Message::new(git_message)).await?;
    println!("{}", response.content);

    if let Some(branch_name) = branch {
        service.process_message(Message::new(format!("branch {}", branch_name))).await?;
    }

    if let Some(target_branch) = merge {
        service.process_message(Message::new(format!("merge {}", target_branch))).await?;
    }

    Ok(())
}

async fn handle_init_command(service: &mut TransferService, project_type: String, name: String, description: String) -> Result<(), Error> {
    service.set_current_agent("project".to_string());

    let init_message = format!("create {} {} '{}'", project_type, name, description);
    let response = service.process_message(Message::new(init_message)).await?;
    println!("{}", response.content);

    Ok(())
}

async fn interactive_mode(service: &mut TransferService, registry: Arc<RwLock<agents::AgentRegistry>>) -> Result<(), Error> {
    println!("🌟 Welcome to the Quantum Swarm CLI! 🌟");
    println!("Available agents:");

    let agents = registry.read().await;
    for (name, _) in agents.iter() {
        println!("- {}", name);
    }

    loop {
        let current_agent_display = match service.get_current_agent() {
            Some(agent) => agent,
            None => "No current agent",
        };
        println!("\nCurrent agent: {}", current_agent_display);
        print!("> ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let message = Message::new(input.trim().to_string());
        let response = service.process_message(message).await?;

        println!("{}", response.content);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

async fn run() -> Result<(), Error> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Initialize the agent registry.
    initialize_registry().await?;

    // Create transfer service starting with greeter.
    let mut service = TransferService::new(agents::GLOBAL_REGISTRY.clone());
    service.set_current_agent("greeter".to_string());

    match cli.command {
        Some(Commands::Git { message, branch, merge }) => {
            // TODO: Implement handle_git_command
            println!("Git command not yet implemented");
        }
        Some(Commands::Init { project_type, name, description }) => {
            // TODO: Implement handle_init_command
            println!("Init command not yet implemented");
        }
        None => {
            // TODO: Implement interactive_mode
            println!("Interactive mode not yet implemented");
        }
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
