use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    api::AppState,
    types::{Message, AgentConfig, Agent, AgentInfo, TodoTask, TaskPriority, TaskStatus, TodoProcessor},
    agents::AgentRegistry,
};

use super::models::TaskResponse;

pub async fn index() -> Response {
    "Welcome to the Swarmonomicon API".into_response()
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    name: String,
    description: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    content: String,
}

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AgentInfo>>, StatusCode> {
    let registry = state.agents.read().await;
    let mut agents = Vec::new();

    for (name, agent) in registry.agents.iter() {
        match agent.get_config().await {
            Ok(config) => agents.push(AgentInfo {
                name: name.clone(),
                description: config.public_description,
                tools: config.tools,
                downstream_agents: config.downstream_agents,
            }),
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<AgentInfo>, StatusCode> {
    let registry = state.agents.read().await;

    if let Some(agent) = registry.get(&name) {
        match agent.get_config().await {
            Ok(config) => Ok(Json(AgentInfo {
                name: config.name,
                description: config.public_description,
                tools: config.tools,
                downstream_agents: config.downstream_agents,
            })),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn process_message(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<Message>, StatusCode> {
    let registry = state.agents.read().await;

    if let Some(agent) = registry.get(&agent_name) {
        match agent.process_message(Message::new(request.content)).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<Message>, StatusCode> {
    let registry = state.agents.read().await;

    if let Some(agent) = registry.get(&agent_name) {
        match agent.process_message(Message::new(request.content)).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn default_agents() -> Vec<AgentConfig> {
    // vec![ restore default later ?
    //     AgentConfig {
    //         name: "greeter".to_string(),
    //         public_description: "Agent that greets the user.".to_string(),
    //         instructions: "Please greet the user to the Swarmonomicon project.".to_string(),
    //         tools: Vec::new(),
    //         downstream_agents: vec!["haiku".to_string()],
    //         personality: None,
    //         state_machine: None,
    //     }
    // ]
    let mut agents = Vec::new();

    #[cfg(feature = "greeter-agent")]
    agents.push(AgentConfig {
        name: "greeter".to_string(),
        public_description: "Agent that greets the user.".to_string(),
        instructions: "Please greet the user and ask them if they'd like a Haiku. If yes, transfer them to the 'haiku' agent.".to_string(),
        tools: Vec::new(),
        downstream_agents: vec!["haiku".to_string()],
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "haiku-agent")]
    agents.push(AgentConfig {
        name: "haiku".to_string(),
        public_description: "Agent that creates haikus.".to_string(),
        instructions: "Create haikus based on user input.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "git-agent")]
    agents.push(AgentConfig {
        name: "git".to_string(),
        public_description: "Agent that helps with git operations.".to_string(),
        instructions: "Help users with git operations like commit, branch, merge etc.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "project-init-agent")]
    agents.push(AgentConfig {
        name: "project-init".to_string(),
        public_description: "Agent that helps initialize new projects.".to_string(),
        instructions: "Help users create new projects with proper structure and configuration.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "browser-agent")]
    agents.push(AgentConfig {
        name: "browser".to_string(),
        public_description: "Agent that controls browser automation.".to_string(),
        instructions: "Help users with browser automation tasks.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    agents
}

#[derive(Debug, Deserialize, Clone)]
pub struct AddTaskRequest {
    pub description: String,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
}

// Get all tasks for an agent
pub async fn get_tasks(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
) -> Result<Json<Vec<TaskResponse>>, StatusCode> {
    let registry = state.agents.read().await;
    
    let agent = registry.get(&agent_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let todo_list = <dyn Agent>::get_todo_list(agent)
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    let tasks = todo_list.get_all_tasks().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(tasks.into_iter().map(TaskResponse::from).collect()))
}

// Get a specific task by ID
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path((agent_name, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let registry = state.agents.read().await;
    
    let agent = registry.get(&agent_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let todo_list = <dyn Agent>::get_todo_list(agent)
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    let task = todo_list.get_task(&task_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(TaskResponse::from(task)))
}

// Add a task to an agent's todo list
pub async fn add_task(
    State(state): State<Arc<AppState>>,
    Path(agent_name): Path<String>,
    Json(request): Json<AddTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let registry = state.agents.read().await;
    
    let agent = registry.get(&agent_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let todo_list = <dyn Agent>::get_todo_list(agent)
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    let task = TodoTask {
        id: uuid::Uuid::new_v4().to_string(),
        description: request.description,
        priority: request.priority,
        source_agent: request.source_agent,
        target_agent: agent_name,
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now().timestamp(),
        completed_at: None,
    };
    
    todo_list.add_task(task.clone()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(TaskResponse::from(task)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentRegistry, GreeterAgent, TransferService};
    use crate::types::AgentConfig;

    #[tokio::test]
    async fn test_list_agents() {
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register("test".to_string(), Box::new(agent)).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let state = Arc::new(AppState {
            transfer_service: Arc::new(RwLock::new(TransferService::new(registry.clone()))),
            agents: registry,
        });

        let response = list_agents(State(state)).await.unwrap();
        assert_eq!(response.0.len(), 1);
        assert_eq!(response.0[0].name, "test");
    }

    #[tokio::test]
    async fn test_get_agent() {
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });
        let response = get_agent(State(state.clone()), Path("unknown".to_string())).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_send_message() {
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register("test".to_string(), Box::new(agent)).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let state = Arc::new(AppState {
            transfer_service: Arc::new(RwLock::new(TransferService::new(registry.clone()))),
            agents: registry,
        });

        let request = MessageRequest {
            content: "Hello".to_string(),
        };

        let response = send_message(
            State(state),
            Path("test".to_string()),
            Json(request),
        ).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_todo_list_endpoints() {
        // Create test state
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test_agent".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register("test_agent".to_string(), Box::new(agent)).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let transfer_service = Arc::new(RwLock::new(TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });

        // Test adding a task
        let add_request = AddTaskRequest {
            description: "Test task".to_string(),
            priority: TaskPriority::Medium,
            source_agent: None,
        };

        let response = add_task(
            State(state.clone()),
            Path("test_agent".to_string()),
            Json(add_request.clone()),
        ).await.unwrap();

        let task = response.0;
        assert_eq!(task.description, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);

        // Test getting all tasks
        let tasks = get_tasks(
            State(state.clone()),
            Path("test_agent".to_string()),
        ).await.unwrap();

        assert_eq!(tasks.0.len(), 1);
        assert_eq!(tasks.0[0].description, "Test task");

        // Test getting a specific task
        let task = get_task(
            State(state.clone()),
            Path(("test_agent".to_string(), task.id.clone())),
        ).await.unwrap();

        assert_eq!(task.0.description, "Test task");
        assert_eq!(task.0.id, task.0.id);

        // Test getting a non-existent task
        let result = get_task(
            State(state.clone()),
            Path(("test_agent".to_string(), "non-existent".to_string())),
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);

        // Test adding a task to a non-existent agent
        let result = add_task(
            State(state.clone()),
            Path("non-existent".to_string()),
            Json(add_request),
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }
}
