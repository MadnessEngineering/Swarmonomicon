use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use super::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub status: TaskStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct TodoList {
    tasks: Arc<RwLock<VecDeque<TodoTask>>>,
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn add_task(&self, task: TodoTask) {
        let mut tasks = self.tasks.write().await;
        tasks.push_back(task);
    }

    pub async fn get_next_task(&self) -> Option<TodoTask> {
        let mut tasks = self.tasks.write().await;
        tasks.pop_front()
    }

    pub async fn peek_next_task(&self) -> Option<TodoTask> {
        let tasks = self.tasks.read().await;
        tasks.front().cloned()
    }

    pub async fn mark_task_completed(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(chrono::Utc::now().timestamp());
        }
    }

    pub async fn mark_task_failed(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Failed;
        }
    }

    pub async fn is_empty(&self) -> bool {
        let tasks = self.tasks.read().await;
        tasks.is_empty()
    }

    pub async fn len(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.len()
    }
}

#[async_trait::async_trait]
pub trait TodoProcessor: Send + Sync {
    /// Process a single task from the todo list
    async fn process_task(&self, task: TodoTask) -> super::Result<Message>;
    
    /// Get the interval at which this processor should check for new tasks
    fn get_check_interval(&self) -> std::time::Duration;
    
    /// Get the todo list for this processor
    fn get_todo_list(&self) -> &TodoList;
    
    /// Start the task processing loop
    async fn start_processing(&self) -> super::Result<()> {
        loop {
            if let Some(task) = self.get_todo_list().get_next_task().await {
                match self.process_task(task.clone()).await {
                    Ok(_) => {
                        self.get_todo_list().mark_task_completed(&task.id).await;
                    }
                    Err(_) => {
                        self.get_todo_list().mark_task_failed(&task.id).await;
                    }
                }
            }
            tokio::time::sleep(self.get_check_interval()).await;
        }
    }
} 
