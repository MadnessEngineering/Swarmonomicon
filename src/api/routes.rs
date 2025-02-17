use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Duration;
use futures::executor::block_on;
use async_trait::async_trait;
use anyhow::anyhow;
use mongodb::{Client, Collection};

use crate::{
    api::AppState,
    types::{Message, AgentConfig, Agent, AgentInfo, TodoTask, TaskPriority, TaskStatus, TodoProcessor, TodoList, StateMachine, AgentStateManager, Tool},
    agents::AgentRegistry,
    ai::{AiProvider, DefaultAiClient},
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
        enhanced_description: None,  // No AI enhancement at this level
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
    use crate::types::{Message, State, StateMachine, AgentStateManager, TodoProcessor, TodoTask};
    use crate::types::todo::{TaskStatus, TaskPriority, TodoList};
    use std::time::Duration;
    use futures::executor::block_on;
    use crate::agents::{AgentRegistry, GreeterAgent, TransferService};
    use mongodb::{Client, Collection};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use axum::http::StatusCode;
    use axum::extract::Path;
    use axum::Json;

    struct MockAiClient;

    #[async_trait]
    impl AiProvider for MockAiClient {
        async fn chat(&self, _system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String, anyhow::Error> {
            // Return a mock enhanced description that includes the original content
            let content = messages.last()
                .and_then(|m| m.get("content"))
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(format!("Enhanced: {}", content))
        }
    }

    struct TestAgent {
        config: AgentConfig,
        todo_list: TodoList,
        ai_client: Arc<Box<dyn AiProvider + Send + Sync>>,
        _client: Arc<Client>, // Keep the MongoDB client alive
    }

    impl TestAgent {
        async fn new_with_mocks(config: AgentConfig, client: Arc<Client>) -> Result<Self, anyhow::Error> {
            // Set up MongoDB connection for testing
            std::env::set_var("RTK_MONGO_URI", "mongodb://localhost:27017/swarmonomicon_test");
            
            // Create a new TodoList that will use the test database
            let todo_list = TodoList::new().await?;
            
            Ok(Self {
                config: config.clone(),
                todo_list,
                ai_client: Arc::new(Box::new(MockAiClient) as Box<dyn AiProvider + Send + Sync>),
                _client: client,
            })
        }

        async fn enhance_task_description(&self, description: String) -> Result<String, anyhow::Error> {
            let prompt = format!(
                "Enhance this task description while maintaining its core meaning: {}",
                description
            );
            let response = self.ai_client.chat("You are a task description enhancer", vec![
                HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    ("content".to_string(), prompt),
                ])
            ]).await?;
            Ok(response)
        }
    }

    #[async_trait]
    impl Agent for TestAgent {
        async fn process_message(&self, message: Message) -> Result<Message, anyhow::Error> {
            Ok(Message::new("Test response".to_string()))
        }

        async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message, anyhow::Error> {
            if !self.config.downstream_agents.contains(&target_agent) {
                return Err(anyhow!("Cannot transfer to unknown agent: {}", target_agent));
            }
            Ok(Message::new(format!("Transferring to {}", target_agent)))
        }

        async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String, anyhow::Error> {
            Ok("Tool called".to_string())
        }

        async fn get_current_state(&self) -> Result<Option<State>, anyhow::Error> {
            Ok(None)
        }

        async fn get_config(&self) -> Result<AgentConfig, anyhow::Error> {
            Ok(self.config.clone())
        }

        fn get_todo_list(&self) -> Option<&TodoList> {
            Some(&self.todo_list)
        }
    }

    #[async_trait]
    impl TodoProcessor for TestAgent {
        async fn process_task(&self, task: TodoTask) -> Result<Message, anyhow::Error> {
            // Enhance the task description using AI
            let enhanced_description = self.enhance_task_description(task.description.clone()).await?;
            
            // Create a new task with the enhanced description
            let enhanced_task = TodoTask {
                description: enhanced_description,
                ..task
            };
            
            // Add the enhanced task to the todo list
            self.todo_list.add_task(enhanced_task.clone()).await.map_err(|e| anyhow!("Failed to add task: {}", e))?;
            
            Ok(Message::new(format!("Processed task: {}", enhanced_task.description)))
        }

        fn get_check_interval(&self) -> Duration {
            Duration::from_secs(5)
        }

        fn get_todo_list(&self) -> &TodoList {
            &self.todo_list
        }
    }

    #[tokio::test]
    async fn test_todo_list_endpoints() -> Result<(), anyhow::Error> {
        // Set up MongoDB connection for testing
        let client = Arc::new(Client::with_uri_str("mongodb://localhost:27017").await?);
        let db = client.database("swarmonomicon_test");
        
        // Clean up any existing data in the test database
        db.collection::<TodoTask>("todos").drop(None).await?;

        // Set up test environment
        let mut registry = AgentRegistry::new();
        let agent = TestAgent::new_with_mocks(AgentConfig {
            name: "test_agent".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec!["haiku".to_string()],
            personality: None,
            state_machine: None,
        }, client.clone()).await?;

        registry.register("test_agent".to_string(), Box::new(agent)).await?;
        let registry = Arc::new(RwLock::new(registry));
        let transfer_service = Arc::new(RwLock::new(crate::agents::TransferService::new(registry.clone())));
        let state = Arc::new(AppState {
            transfer_service,
            agents: registry,
        });

        // Test 1: Add a task with AI enhancement
        let add_request = AddTaskRequest {
            description: "Write a function to calculate fibonacci numbers".to_string(),
            priority: TaskPriority::High,
            source_agent: Some("user".to_string()),
        };

        let response = add_task(
            State(state.clone()),
            Path("test_agent".to_string()),
            Json(add_request.clone()),
        ).await.map_err(|e| anyhow!("Failed to add task: {:?}", e))?;

        let task_id = response.0.id.clone();
        let task_description = response.0.description.clone();
        
        assert_eq!(response.0.priority, TaskPriority::High);
        assert!(task_description.contains("fibonacci"), "AI-enhanced description should maintain core meaning");
        assert_eq!(response.0.status, TaskStatus::Pending);

        // Test 2: Add multiple tasks and verify prioritization
        let low_priority_task = AddTaskRequest {
            description: "Update documentation".to_string(),
            priority: TaskPriority::Low,
            source_agent: None,
        };

        let medium_priority_task = AddTaskRequest {
            description: "Add error handling".to_string(),
            priority: TaskPriority::Medium,
            source_agent: None,
        };

        add_task(
            State(state.clone()),
            Path("test_agent".to_string()),
            Json(low_priority_task),
        ).await.map_err(|e| anyhow!("Failed to add low priority task: {:?}", e))?;

        add_task(
            State(state.clone()),
            Path("test_agent".to_string()),
            Json(medium_priority_task),
        ).await.map_err(|e| anyhow!("Failed to add medium priority task: {:?}", e))?;

        // Test 3: Get all tasks and verify ordering
        let tasks = get_tasks(
            State(state.clone()),
            Path("test_agent".to_string()),
        ).await.map_err(|e| anyhow!("Failed to get tasks: {:?}", e))?;

        assert_eq!(tasks.0.len(), 3);
        assert_eq!(tasks.0[0].priority, TaskPriority::High); // Should be first due to priority

        // Test 4: Get specific task and verify details
        let task = get_task(
            State(state.clone()),
            Path(("test_agent".to_string(), task_id.clone())),
        ).await.map_err(|e| anyhow!("Failed to get specific task: {:?}", e))?;

        assert_eq!(task.0.description, task_description);
        assert_eq!(task.0.id, task_id);

        // Test 5: Error handling for non-existent task
        let result = get_task(
            State(state.clone()),
            Path(("test_agent".to_string(), "non-existent".to_string())),
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);

        // Test 6: Error handling for non-existent agent
        let result = add_task(
            State(state.clone()),
            Path("non-existent".to_string()),
            Json(add_request),
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);

        // Test 7: Task delegation between agents
        let delegated_task = AddTaskRequest {
            description: "Create a haiku about coding".to_string(),
            priority: TaskPriority::Medium,
            source_agent: Some("test_agent".to_string()),
        };

        let response = add_task(
            State(state.clone()),
            Path("haiku".to_string()),
            Json(delegated_task),
        ).await;

        assert!(response.is_err()); // Should fail since haiku agent isn't registered
        assert_eq!(response.unwrap_err(), StatusCode::NOT_FOUND);

        // Clean up test database
        db.collection::<TodoTask>("todos").drop(None).await?;

        Ok(())
    }
}
