use std::collections::HashMap;
use anyhow::Result;
use crate::types::TaskPriority;

mod goose;
mod local;

pub use goose::GooseClient;
pub use local::LocalAiClient;

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String>;
}

// Re-export the default client based on feature flags
#[cfg(feature = "goose")]
pub type DefaultAiClient = GooseClient;

#[cfg(not(feature = "goose"))]
pub type DefaultAiClient = LocalAiClient;

// Helper function to create a new AI client
pub fn new_ai_client() -> DefaultAiClient {
    DefaultAiClient::new()
}

/// Enhances a todo description using AI, predicting priority and project
///
/// Returns a tuple of (enhanced_description, priority, project_name)
pub async fn enhance_todo_description(
    description: &str,
    ai_client: &dyn AiProvider
) -> Result<(String, TaskPriority, String)> {
    // Enhance the description
    let system_prompt = r#"You are a task enhancement and planning system. Imagine you are creating a prompt for an ai agent to complete the Task given:
1. Adding specific technical steps to complete the task
2. Explaining impact and scope, along with file locations and dependencies
3. Including relevant components/systems that are involved
4. Break up tasks into smaller steps to control context length
5. Keeping it concise
6. Use markdown formatting for the output

Output ONLY the enhanced description, no other text."#;

    let messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Enhance this task: {}", description)),
    ])];

    let enhanced_description = ai_client.chat(system_prompt, messages).await?;

    // Predict task priority
    let priority_prompt = r#"You are a task priority classifier. Analyze the task and determine its priority level.
Output ONLY one of these priority levels, with no other text: "inital", "high", "medium", or "low".
Use these guidelines:
- Inital: Tasks that are new and not yet able to be compared to other tasks
- High: Important tasks that significantly impact functionality or performance
- Medium: Standard development work or minor improvements
- Low: Nice to have features, documentation, or cosmetic issues"#;

    let priority_messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Classify priority: {}", description)),
    ])];

    let priority_response = ai_client.chat(priority_prompt, priority_messages).await?;
    let priority = match priority_response.trim().to_lowercase().as_str() {
        "inital" => TaskPriority::Inital,
        "high" => TaskPriority::High,
        "medium" => TaskPriority::Medium,
        "low" => TaskPriority::Low,
        _ => TaskPriority::Medium, // Default to Medium for any unexpected response
    };

    // Predict project
    let project_prompt = r#"You are a project classifier. Your task is to determine which project a given task belongs to. 
Your output should be ONLY the project name, nothing else. Options are: 
"madness_interactive - Parent Project of chaos", 
"regressiontestkit - Parent repo for Work projects. Balena device testing in python", 
"omnispindle - MCP server for Managing AI todo list in python", 
"Todomill_projectorium - Todo list management Dashbaord on Node-red",
"swarmonomicon - Todo worker and generation project in rust", 
"hammerspoon - MacOS automation and workspace management", 
"lab_management - Lab management general project", 
"cogwyrm - Mobile app for Tasker infacing with madness network", 
"docker_implementation - Tasks todo with docker and deployment", 
"documentation - Documentation for all projects", 
"eventghost - Event handling and monitoring automation. Being rewritten in Rust", 
"hammerghost - MacOS automation menu in hammerspoon based on eventghost",  
"quality_assurance - Quality assurance tasks",
"spindlewrit - Writing and documentation project",
"inventorium - Madnessinteractice.cc website and Todo Dashboard - React",

If you're unsure, default to "madness_interactive"."#;

    let project_messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Which project does this task belong to? {}", description)),
    ])];

    let project_name = ai_client.chat(project_prompt, project_messages).await?;

    // Verify project name against valid options
    let valid_projects = [
        "madness_interactive",
        "regressiontestkit",
        "omnispindle",
        "todomill_projectorium",
        "swarmonomicon",
        "hammerspoon",
        "lab_management",
        "cogwyrm",
        "docker_implementation",
        "documentation",
        "eventghost",
        "hammerghost",
        "quality_assurance",
        "spindlewrit",
        "inventorium"
    ];

    // Clean up project name
    let project = project_name.trim().trim_matches('"').trim_matches('\'').to_lowercase();

    // Verify project exists in valid list
    let verified_project = if valid_projects.iter().any(|&p| p == project) {
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
        verified_project.to_string()
    };

    Ok((enhanced_description, priority, final_project))
}

// Deprecated: Use new_ai_client() instead
#[deprecated(since = "0.1.0", note = "please use `new_ai_client()` instead")]
pub use self::local::LocalAiClient as AiClient;
