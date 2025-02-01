use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use super::Message;
use std::time::Duration;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use super::Result;
use crate::agents::wrapper::AgentWrapper;
use crate::types::Agent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone)]
pub struct TodoList {
    tasks: HashMap<String, TodoTask>,
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: TodoTask) {
        self.tasks.insert(task.id.clone(), task);
    }

    pub fn get_tasks(&self) -> Vec<TodoTask> {
        self.tasks.values().cloned().collect()
    }

    pub fn get_task(&self, id: &str) -> Option<&TodoTask> {
        self.tasks.get(id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut TodoTask> {
        self.tasks.get_mut(id)
    }

    pub fn get_next_task(&self) -> Option<TodoTask> {
        self.tasks.values()
            .find(|t| t.status == TaskStatus::Pending)
            .cloned()
    }

    pub fn mark_task_completed(&mut self, id: &str) {
        if let Some(task) = self.get_task_mut(id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(Utc::now());
        }
    }

    pub fn mark_task_failed(&mut self, id: &str) {
        if let Some(task) = self.get_task_mut(id) {
            task.status = TaskStatus::Failed;
        }
    }

    pub fn update_task(&mut self, id: &str, task: TodoTask) -> Result<()> {
        if self.tasks.contains_key(id) {
            self.tasks.insert(id.to_string(), task);
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    pub fn delete_task(&mut self, id: &str) -> Result<()> {
        if self.tasks.remove(id).is_some() {
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }
}

#[async_trait]
pub trait TodoProcessor: Send + Sync {
    async fn process_task(&mut self, task: TodoTask) -> Result<Message>;
    async fn get_todo_list(&self) -> Arc<RwLock<TodoList>>;
    async fn start_processing(&mut self);
    fn get_check_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

#[async_trait]
pub trait TodoListExt {
    async fn add_task(&self, task: TodoTask) -> Result<()>;
    async fn get_task(&self, id: &str) -> Option<TodoTask>;
    async fn get_tasks(&self) -> Vec<TodoTask>;
    async fn update_task(&self, id: &str, task: TodoTask) -> Result<()>;
    async fn delete_task(&self, id: &str) -> Result<()>;
}

#[async_trait]
impl TodoListExt for Arc<RwLock<TodoList>> {
    async fn add_task(&self, task: TodoTask) -> Result<()> {
        let mut list = self.write().await;
        list.add_task(task);
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Option<TodoTask> {
        let list = self.read().await;
        list.get_task(id).cloned()
    }

    async fn get_tasks(&self) -> Vec<TodoTask> {
        let list = self.read().await;
        list.get_tasks()
    }

    async fn update_task(&self, id: &str, task: TodoTask) -> Result<()> {
        let mut list = self.write().await;
        list.update_task(id, task)?;
        Ok(())
    }

    async fn delete_task(&self, id: &str) -> Result<()> {
        let mut list = self.write().await;
        list.delete_task(id)?;
        Ok(())
    }
}

#[async_trait]
impl TodoProcessor for AgentWrapper {
    async fn process_task(&mut self, task: TodoTask) -> Result<Message> {
        let task_desc = task.description.clone();
        let result = self.process_message(Message::new(task_desc)).await;
        if result.is_ok() {
            let mut list = self.todo_list.write().await;
            list.mark_task_completed(&task.id);
        } else {
            let mut list = self.todo_list.write().await;
            list.mark_task_failed(&task.id);
        }
        result
    }

    async fn get_todo_list(&self) -> Arc<RwLock<TodoList>> {
        self.todo_list.clone()
    }

    async fn start_processing(&mut self) {
        loop {
            let todo_list = TodoProcessor::get_todo_list(self).await;
            let mut list = todo_list.write().await;
            
            if let Some(task) = list.get_next_task() {
                let task_id = task.id.clone();
                drop(list); // Release the lock before processing
                let result = self.process_task(task).await;
                let mut list = todo_list.write().await;
                if result.is_ok() {
                    list.mark_task_completed(&task_id);
                } else {
                    list.mark_task_failed(&task_id);
                }
            } else {
                drop(list);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
} 
