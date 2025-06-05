use std::fs;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, Tool, ToolCall, State, TaskPriority};
use crate::tools::ToolRegistry;
use crate::ai::{AiProvider, DefaultAiClient};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};
use std::error::Error as StdError;
use anyhow::{Result as AnyhowResult, anyhow};

// Project classification request/response structures for MQTT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectClassificationRequest {
    pub description: String,
    pub request_id: Option<String>,
    pub context: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectClassificationResponse {
    pub project_name: String,
    pub confidence: f64,
    pub request_id: Option<String>,
    pub reasoning: Option<String>,
}

// Background task tracking
#[derive(Debug, Clone)]
struct BackgroundTask {
    pub id: String,
    pub task_type: BackgroundTaskType,
    pub project: String,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
enum BackgroundTaskType {
    GitCommitAnalysis,
    ProjectMaintenance,
    DependencyUpdates,
    DocumentationSync,
}

#[derive(Debug, Clone)]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

pub struct ProjectAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    current_state: Option<String>,
    ai_client: Arc<dyn AiProvider + Send + Sync>,
    background_tasks: Arc<RwLock<Vec<BackgroundTask>>>,
    last_git_check: Arc<Mutex<Instant>>,
    valid_projects: Vec<String>,
}

impl ProjectAgent {
    pub async fn new(config: AgentConfig) -> Result<Self> {
        let valid_projects = vec![
            "madness_interactive".to_string(),
            "regressiontestkit".to_string(),
            "omnispindle".to_string(),
            "todomill_projectorium".to_string(),
            "swarmonomicon".to_string(),
            "hammerspoon".to_string(),
            "lab_management".to_string(),
            "cogwyrm".to_string(),
            "docker_implementation".to_string(),
            "documentation".to_string(),
            "eventghost".to_string(),
            "hammerghost".to_string(),
            "quality_assurance".to_string(),
            "spindlewrit".to_string(),
            "node_red_contrib_file_template".to_string(),
            "inventorium".to_string(),
        ];

        let agent = Self {
            config,
            tools: ToolRegistry::create_default_tools().await?,
            current_state: None,
            ai_client: Arc::new(DefaultAiClient::new()),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
            last_git_check: Arc::new(Mutex::new(Instant::now())),
            valid_projects,
        };

        // Initialize background tasks
        agent.setup_background_tasks().await?;

        Ok(agent)
    }

    /// Classify a project description and return the project name
    pub async fn classify_project(&self, request: ProjectClassificationRequest) -> Result<ProjectClassificationResponse> {
        let project_prompt = r#"You are a project classifier. Your task is to determine which project a given task belongs to. 
Your output should be ONLY the project name, nothing else. Options are: 
"madness_interactive - Parent Project of chaos", 
"regressiontestkit - Parent repo for Work projects. Balena device testing in python", 
"omnispindle - MCP server for Managing AI todo list in python", 
"todomill_projectorium - Todo list management Dashboard on Node-red",
"swarmonomicon - Todo worker and generation project in rust", 
"hammerspoon - MacOS automation and workspace management", 
"lab_management - Lab management general project", 
"cogwyrm - Mobile app for Tasker interfacing with madness network", 
"docker_implementation - Tasks todo with docker and deployment", 
"documentation - Documentation for all projects", 
"eventghost - Event handling and monitoring automation. Being rewritten in Rust", 
"hammerghost - MacOS automation menu in hammerspoon based on eventghost",  
"quality_assurance - Quality assurance tasks",
"spindlewrit - Writing and documentation project",
"node_red_contrib_file_template - Node-red contrib for file management replacement of the HTML template node",
"inventorium - Madnessinteractive.cc website and Todo Dashboard - React",

If you're unsure, default to "madness_interactive"."#;

        let messages = vec![HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), format!("Which project does this task belong to? {}", request.description)),
        ])];

        let project_name = self.ai_client.chat(project_prompt, messages).await?;

        // Clean up project name
        let project = project_name.trim().trim_matches('"').trim_matches('\'').to_lowercase();

        // Verify project exists in valid list
        let verified_project = if self.valid_projects.iter().any(|p| p == &project) {
            project
        } else {
            // If not a valid project, default to madness_interactive
            log::warn!("Invalid project name detected: '{}'. Defaulting to madness_interactive", project);
            "madness_interactive".to_string()
        };

        // If project is empty, use default
        let final_project = if verified_project.is_empty() {
            "madness_interactive".to_string()
        } else {
            verified_project
        };

        // Schedule background work for this project
        self.schedule_project_background_work(&final_project).await?;

        Ok(ProjectClassificationResponse {
            project_name: final_project,
            confidence: 0.8, // TODO: Implement actual confidence scoring
            request_id: request.request_id,
            reasoning: Some(format!("Classified based on keywords and context analysis")),
        })
    }

    /// Schedule background work for a project
    async fn schedule_project_background_work(&self, project: &str) -> Result<()> {
        let mut tasks = self.background_tasks.write().await;
        
        // Schedule git commit analysis
        let git_task = BackgroundTask {
            id: format!("git-analysis-{}-{}", project, Utc::now().timestamp()),
            task_type: BackgroundTaskType::GitCommitAnalysis,
            project: project.to_string(),
            created_at: Utc::now(),
            last_run: None,
            next_run: Utc::now() + chrono::Duration::minutes(5), // Run in 5 minutes
            status: TaskStatus::Pending,
        };
        
        // Schedule project maintenance
        let maintenance_task = BackgroundTask {
            id: format!("maintenance-{}-{}", project, Utc::now().timestamp()),
            task_type: BackgroundTaskType::ProjectMaintenance,
            project: project.to_string(),
            created_at: Utc::now(),
            last_run: None,
            next_run: Utc::now() + chrono::Duration::hours(1), // Run in 1 hour
            status: TaskStatus::Pending,
        };

        tasks.push(git_task);
        tasks.push(maintenance_task);

        // Start background task processing if not already running
        let background_tasks = self.background_tasks.clone();
        tokio::spawn(async move {
            Self::process_background_tasks(background_tasks).await;
        });

        Ok(())
    }

    /// Setup initial background tasks
    async fn setup_background_tasks(&self) -> Result<()> {
        // Initialize periodic tasks for all projects
        for project in &self.valid_projects {
            self.schedule_project_background_work(project).await?;
        }
        Ok(())
    }

    /// Process background tasks continuously
    async fn process_background_tasks(tasks: Arc<RwLock<Vec<BackgroundTask>>>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
        
        loop {
            interval.tick().await;
            
            let mut tasks_guard = tasks.write().await;
            let now = Utc::now();
            
            for task in tasks_guard.iter_mut() {
                if matches!(task.status, TaskStatus::Pending) && task.next_run <= now {
                    task.status = TaskStatus::Running;
                    
                    // Clone task for processing
                    let task_clone = task.clone();
                    
                    // Process task in background
                    tokio::spawn(async move {
                        let result = Self::execute_background_task(&task_clone).await;
                        log::info!("Background task {} completed: {:?}", task_clone.id, result);
                    });
                    
                    // Update task timing
                    task.last_run = Some(now);
                    task.next_run = now + chrono::Duration::hours(24); // Daily by default
                    task.status = TaskStatus::Completed;
                }
            }
        }
    }

    /// Execute a specific background task
    async fn execute_background_task(task: &BackgroundTask) -> Result<()> {
        match task.task_type {
            BackgroundTaskType::GitCommitAnalysis => {
                Self::analyze_git_commits(&task.project).await
            },
            BackgroundTaskType::ProjectMaintenance => {
                Self::perform_project_maintenance(&task.project).await
            },
            BackgroundTaskType::DependencyUpdates => {
                Self::check_dependency_updates(&task.project).await
            },
            BackgroundTaskType::DocumentationSync => {
                Self::sync_documentation(&task.project).await
            },
        }
    }

    /// Analyze recent git commits for a project
    async fn analyze_git_commits(project: &str) -> Result<()> {
        log::info!("Analyzing git commits for project: {}", project);
        
        // Check if we're in a git repository
        let output = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()?;
            
        if !output.status.success() {
            log::warn!("Not in a git repository, skipping git analysis for {}", project);
            return Ok(());
        }

        // Get recent commits (last 24 hours)
        let output = Command::new("git")
            .args(["log", "--oneline", "--since=24 hours ago"])
            .output()?;
            
        if output.status.success() {
            let commits = String::from_utf8_lossy(&output.stdout);
            if !commits.trim().is_empty() {
                log::info!("Recent commits for {}: {}", project, commits);
                // TODO: Analyze commits and create related todos
                // This could involve:
                // - Finding related issues/todos
                // - Checking for TODO comments in code
                // - Identifying areas that need attention
            }
        }
        
        Ok(())
    }

    /// Perform general project maintenance
    async fn perform_project_maintenance(project: &str) -> Result<()> {
        log::info!("Performing maintenance for project: {}", project);
        
        // Check for common maintenance tasks
        // TODO: Implement specific maintenance tasks:
        // - Check for outdated dependencies
        // - Review code quality metrics
        // - Update documentation
        // - Clean up temporary files
        // - Check CI/CD pipeline status
        
        Ok(())
    }

    /// Check for dependency updates
    async fn check_dependency_updates(project: &str) -> Result<()> {
        log::info!("Checking dependency updates for project: {}", project);
        
        // Check for different project types
        if Path::new("Cargo.toml").exists() {
            // Rust project
            let output = Command::new("cargo")
                .args(["outdated"])
                .output();
                
            if let Ok(output) = output {
                if output.status.success() {
                    let outdated = String::from_utf8_lossy(&output.stdout);
                    if !outdated.trim().is_empty() {
                        log::info!("Outdated dependencies in {}: {}", project, outdated);
                    }
                }
            }
        } else if Path::new("requirements.txt").exists() {
            // Python project
            let output = Command::new("pip")
                .args(["list", "--outdated"])
                .output();
                
            if let Ok(output) = output {
                if output.status.success() {
                    let outdated = String::from_utf8_lossy(&output.stdout);
                    if !outdated.trim().is_empty() {
                        log::info!("Outdated Python packages in {}: {}", project, outdated);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Sync documentation
    async fn sync_documentation(project: &str) -> Result<()> {
        log::info!("Syncing documentation for project: {}", project);
        
        // TODO: Implement documentation sync:
        // - Update README files
        // - Generate API documentation
        // - Check for broken links
        // - Update project metadata
        
        Ok(())
    }

    fn init_python_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        // Use the Spindlewrit CLI if available
        if self.is_spindlewrit_available() {
            return self.use_spindlewrit_cli(name, description, "python", path);
        }

        // Fallback to direct implementation
        // Create project structure
        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(src_dir.join(name))?;
        fs::create_dir_all(src_dir.join("tests"))?;

        // Create __init__.py files
        fs::write(src_dir.join(name).join("__init__.py"), "")?;
        fs::write(src_dir.join("tests").join("__init__.py"), "")?;

        // Create requirements.txt
        fs::write(path.join("requirements.txt"), "# Core dependencies\n")?;

        // Create setup.py
        let setup_content = format!(
            r#"from setuptools import setup, find_packages

setup(
    name="{}",
    version="0.1.0",
    packages=find_packages(where="src"),
    package_dir={{"": "src"}},
    install_requires=[],
    python_requires=">=3.8",
)"#,
            name
        );
        fs::write(path.join("setup.py"), setup_content)?;

        self.create_readme(name, description, "python", path)?;
        Ok(())
    }

    fn init_rust_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        // Use the Spindlewrit CLI if available
        if self.is_spindlewrit_available() {
            return self.use_spindlewrit_cli(name, description, "rust", path);
        }

        // Fallback to direct implementation
        Command::new("cargo")
            .args(["init", "--name", name])
            .current_dir(path)
            .output()?;

        self.create_readme(name, description, "rust", path)?;
        Ok(())
    }

    fn init_common_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        // Use the Spindlewrit CLI if available
        if self.is_spindlewrit_available() {
            return self.use_spindlewrit_cli(name, description, "common", path);
        }

        // Fallback to direct implementation
        fs::create_dir_all(path.join("src"))?;
        fs::create_dir_all(path.join("docs"))?;
        fs::create_dir_all(path.join("examples"))?;

        self.create_readme(name, description, "common", path)?;
        // add init .specstory and run fixchat
        // setup the git hooks and init git project
        Ok(())
    }

    fn init_project_from_todo(&self, todo_id: &str, output_path: &Path) -> Result<()> {
        // Check if Spindlewrit is available
        if !self.is_spindlewrit_available() {
            return Err("Spindlewrit CLI not available. Please install it first.".into());
        }

        // Get the GEMMA_API_KEY from environment
        let api_key = std::env::var("GEMMA_API_KEY").ok();
        let api_key_arg = api_key.map(|key| format!("--api-key={}", key)).unwrap_or_default();

        // Run the spindlewrit command
        let output = Command::new("spindlewrit")
            .args([
                "from-todo",
                "--todo-id",
                todo_id,
                "--output-dir",
                output_path.to_str().unwrap(),
            ])
            .arg(api_key_arg)
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to generate project from todo: {}", error_message).into());
        }

        Ok(())
    }

    // Check if the Spindlewrit CLI is available in the system
    fn is_spindlewrit_available(&self) -> bool {
        Command::new("spindlewrit")
            .arg("--help")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    // Use the Spindlewrit CLI to create a project
    fn use_spindlewrit_cli(&self, name: &str, description: &str, project_type: &str, path: &Path) -> Result<()> {
        let output = Command::new("spindlewrit")
            .args([
                "create",
                "--name",
                name,
                "--description",
                description,
                "--type",
                project_type,
                "--path",
                path.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to generate project: {}", error_message).into());
        }

        Ok(())
    }

    fn create_readme(
        &self,
        name: &str,
        description: &str,
        project_type: &str,
        path: &Path,
    ) -> Result<()> {
        let mut content = format!(
            r#"# {name}

{description}

## Overview

This is a {project_type} project created with the project initialization tool.

## Setup

"#
        );

        match project_type {
            "python" => {
                content.push_str(
                    r#"
1. Create and activate a virtual environment:
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   ```
2. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
"#,
                );
            }
            "rust" => {
                content.push_str(
                    r#"
1. Build the project:
   ```bash
   cargo build
   ```
2. Run tests:
   ```bash
   cargo test
   ```
"#,
                );
            }
            _ => {}
        }

        fs::write(path.join("README.md"), content)?;
        Ok(())
    }
}

#[async_trait]
impl Agent for ProjectAgent {
    async fn process_message(&self, message: Message) -> AnyhowResult<Message> {
        // Check if this is a project classification request
        if let Ok(classification_request) = serde_json::from_str::<ProjectClassificationRequest>(&message.content) {
            // Handle project classification
            match self.classify_project(classification_request).await {
                Ok(response) => {
                    let response_content = serde_json::to_string(&response)?;
                    let mut response_message = Message::new(response_content);
                    
                    if let Some(metadata) = message.metadata {
                        let state = self.current_state.clone().unwrap_or_else(|| "classification".to_string());
                        let metadata = MessageMetadata::new("project_classifier".to_string())
                            .with_personality(vec!["analytical".to_string(), "systematic".to_string()])
                            .with_state(state);
                        response_message.metadata = Some(metadata);
                    }
                    
                    return Ok(response_message);
                }
                Err(e) => {
                    let error_response = json!({
                        "error": e.to_string(),
                        "project_name": "madness_interactive", // fallback
                        "confidence": 0.0
                    });
                    return Ok(Message::new(error_response.to_string()));
                }
            }
        }

        // Handle regular project initialization messages
        let mut response = Message::new(format!("Project agent received: {}", message.content));
        if let Some(metadata) = message.metadata {
            let state = self.current_state.clone().unwrap_or_else(|| "initial".to_string());
            let metadata = MessageMetadata::new("project_init".to_string())
                .with_personality(vec!["helpful".to_string(), "technical".to_string()])
                .with_state(state);
            response.metadata = Some(metadata);
        }
        Ok(response)
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> AnyhowResult<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> AnyhowResult<String> {
        self.tools.execute(tool, params).await.map_err(|e| anyhow!("{}", e))
    }

    async fn get_config(&self) -> AnyhowResult<AgentConfig> {
        Ok(self.config.clone())
    }

    async fn get_current_state(&self) -> AnyhowResult<Option<State>> {
        Ok(self.current_state.clone().map(|s| State {
            name: s,
            data: None,
            prompt: None,
            transitions: None,
            validation: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_project_init() -> Result<()> {
        let config = AgentConfig {
            name: "project-init".to_string(),
            public_description: "Test project init".to_string(),
            instructions: "Test project initialization".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let agent = ProjectAgent::new(config).await?;
        let response = agent.process_message(Message::new("test".to_string())).await?;
        assert!(response.content.contains("Project init received"));
        Ok(())
    }
}
