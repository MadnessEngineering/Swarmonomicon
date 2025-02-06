use crate::types::{TodoTask, TaskPriority, TaskStatus};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub status: TaskStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

impl From<TodoTask> for TaskResponse {
    fn from(task: TodoTask) -> Self {
        Self {
            id: task.id,
            description: task.description,
            priority: task.priority,
            source_agent: task.source_agent,
            target_agent: task.target_agent,
            status: task.status,
            created_at: task.created_at,
            completed_at: task.completed_at,
        }
    }
} 
