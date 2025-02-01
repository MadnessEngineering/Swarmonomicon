use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;
use std::error::Error as StdError;
use crate::types::todo::TodoListExt;
use chrono::{DateTime, Utc};
use axum::handler::Handler;
use axum::debug_handler;

use crate::{
    api::AppState,
    types::{Message, AgentConfig, Agent, AgentInfo, TodoTask, TaskPriority, TaskStatus, TodoProcessor, MessageMetadata},
    agents::AgentRegistry,
};

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AppError {
    #[error("Status: {0}")]
    Status(StatusCode),
    #[error("Agent error: {0}")]
    AgentError(String),
    #[error("Serialization error")]
    SerializationError,
}

impl From<StatusCode> for AppError {
    fn from(status: StatusCode) -> Self {
        AppError::Status(status)
    }
}

impl From<Box<dyn StdError + Send + Sync>> for AppError {
    fn from(err: Box<dyn StdError + Send + Sync>) -> Self {
        AppError::AgentError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        AppError::SerializationError
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Status(status) => status,
            AppError::AgentError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::SerializationError => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

pub async fn index() -> Response {
    "Welcome to the Swarmonomicon API".into_response()
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    name: String,
    description: String,
}

#[derive(Deserialize)]
pub struct MessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, AppError> {
    let registry = state.agents.read().await;
    let agents = registry.list_agents();
    Ok(Json(agents))
}

pub async fn get_agent(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<MessageResponse>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let config = agent.read().await.get_config().await?;
    Ok(Json(MessageResponse {
        content: format!("Agent {} is ready", config.name),
        metadata: None,
    }))
}

#[debug_handler]
pub async fn process_message(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(message): Json<MessageRequest>,
) -> Result<Json<Message>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let mut agent = agent.write().await;
    let response = agent.process_message(Message::text(message.content))
        .await
        .map_err(|_| AppError::Status(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Json(response))
}

pub async fn send_message(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let mut agent_lock = agent.write().await;
    let response = agent_lock.process_message(Message::new(request.content)).await?;
    Ok(Json(MessageResponse {
        content: response.content,
        metadata: Some(serde_json::to_value(response.metadata)?),
    }))
}

pub async fn handle_message(
    Path(agent_name): Path<String>,
    State(registry): State<Arc<RwLock<AgentRegistry>>>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let registry = registry.read().await;
    let agent = registry.get_agent(&agent_name).ok_or(StatusCode::NOT_FOUND)?;
    let mut agent_lock = agent.write().await;
    let response = agent_lock.process_message(Message::new(request.content)).await?;
    Ok(Json(MessageResponse {
        content: response.content,
        metadata: response.metadata.map(|m| serde_json::to_value(m).unwrap_or_default()),
    }))
}

pub async fn handle_todo_list(
    Path(agent_name): Path<String>,
    State(registry): State<Arc<RwLock<AgentRegistry>>>,
    Json(task): Json<TodoTask>,
) -> Result<Json<MessageResponse>, AppError> {
    let registry = registry.read().await;
    let agent = registry.get_agent(&agent_name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let agent = agent.read().await;
    let todo_list = TodoProcessor::get_todo_list(&*agent).await;
    {
        let mut list = todo_list.write().await;
        list.add_task(task);
    }
    Ok(Json(MessageResponse {
        content: "Task added successfully".to_string(),
        metadata: None,
    }))
}

pub fn default_agents() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Agent that greets the user.".to_string(),
            instructions: "Please greet the user to the Swarmonomicon project.".to_string(),
            tools: Vec::new(),
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        }
    ]
}

#[derive(Debug, Deserialize, Clone)]
pub struct AddTaskRequest {
    pub description: String,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
}

impl From<AddTaskRequest> for TodoTask {
    fn from(req: AddTaskRequest) -> Self {
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            description: req.description,
            priority: req.priority,
            source_agent: req.source_agent,
            target_agent: "".to_string(), // Will be set by the handler
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        }
    }
}

#[debug_handler]
pub async fn add_task(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(task): Json<TodoTask>,
) -> Result<Json<TodoTask>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let agent = agent.read().await;
    let todo_list = TodoProcessor::get_todo_list(&*agent).await;
    todo_list.add_task(task.clone()).await.map_err(|_| AppError::Status(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Json(task))
}

#[debug_handler]
pub async fn get_tasks(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<TodoTask>>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let agent = agent.read().await;
    let todo_list = TodoProcessor::get_todo_list(&*agent).await;
    let tasks = {
        let list = todo_list.read().await;
        list.get_tasks()
    };
    Ok(Json(tasks))
}

#[debug_handler]
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path((name, task_id)): Path<(String, String)>,
) -> Result<Json<TodoTask>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.get_agent(&name).ok_or(AppError::Status(StatusCode::NOT_FOUND))?;
    let agent = agent.read().await;
    let todo_list = TodoProcessor::get_todo_list(&*agent).await;
    let task = {
        let list = todo_list.read().await;
        list.get_task(&task_id).cloned().ok_or(AppError::Status(StatusCode::NOT_FOUND))?
    };
    Ok(Json(task))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::greeter::GreeterAgent;
    use axum::http::StatusCode;
    use chrono::Utc;

    async fn setup_test_state() -> Arc<AppState> {
        let mut registry = AgentRegistry::new();
        let config = AgentConfig {
            name: "test_agent".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };
        registry.register("test_agent".to_string(), Box::new(GreeterAgent::new(config)));
        Arc::new(AppState {
            agents: Arc::new(RwLock::new(registry)),
        })
    }

    #[tokio::test]
    async fn test_list_agents() {
        let state = setup_test_state().await;
        let response = list_agents(State(state)).await.unwrap();
        assert!(!response.0.is_empty());
    }

    #[tokio::test]
    async fn test_get_agent() {
        let state = setup_test_state().await;
        let response = get_agent(Path("test_agent".to_string()), State(state)).await.unwrap();
        assert!(response.0.content.contains("ready"));
    }

    #[tokio::test]
    async fn test_send_message() {
        let state = setup_test_state().await;
        let request = MessageRequest {
            content: "Hello".to_string(),
        };
        let response = send_message(
            Path("test_agent".to_string()),
            State(state),
            Json(request),
        )
        .await
        .unwrap();
        assert!(!response.0.content.is_empty());
    }

    #[tokio::test]
    async fn test_todo_list_endpoints() {
        let state = setup_test_state().await;
        
        // Test add task
        let task = TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            description: "Test task".to_string(),
            priority: TaskPriority::Medium,
            source_agent: None,
            target_agent: "test_agent".to_string(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        };
        
        let response = add_task(
            State(state.clone()),
            Path("test_agent".to_string()),
            Json(task.clone()),
        )
        .await
        .unwrap();
        assert!(response.0.id.len() > 0);

        // Test get tasks
        let response = get_tasks(
            State(state.clone()),
            Path("test_agent".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(response.0.len(), 1);

        // Test get task
        let task_id = response.0[0].id.clone();
        let response = get_task(
            State(state),
            Path(("test_agent".to_string(), task_id)),
        )
        .await
        .unwrap();
        assert!(response.0.description.contains("Test task"));
    }
}
