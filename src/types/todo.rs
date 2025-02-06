use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use super::Message;
use mongodb::{Client, Collection, Database};
use mongodb::bson::{doc, DateTime};
use mongodb::error::Error as MongoError;
use futures_util::TryStreamExt;
use std::env;

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

#[derive(Debug, Clone)]
pub struct TodoList {
    collection: Collection<TodoTask>,
}

impl TodoList {
    pub async fn new() -> Result<Self, MongoError> {
        let uri = env::var("RTK_MONGO_URI")
            .expect("RTK_MONGO_URI must be set");

        let client = Client::with_uri_str(&uri).await?;
        let db = client.database("swarmonomicon");
        let collection = db.collection("todos");

        Ok(Self { collection })
    }

    pub async fn add_task(&self, task: TodoTask) -> Result<(), MongoError> {
        self.collection.insert_one(task, None).await?;
        Ok(())
    }

    pub async fn get_next_task(&self) -> Result<Option<TodoTask>, MongoError> {
        let filter = doc! {
            "status": "Pending"
        };
        let update = doc! {
            "$set": {
                "status": "InProgress"
            }
        };
        let options = mongodb::options::FindOneAndUpdateOptions::builder()
            .sort(doc! { "priority": -1, "created_at": 1 })
            .build();

        Ok(self.collection
            .find_one_and_update(filter, update, options)
            .await?)
    }

    pub async fn mark_task_completed(&self, task_id: &str) -> Result<(), MongoError> {
        let filter = doc! {
            "id": task_id
        };
        let update = doc! {
            "$set": {
                "status": "Completed",
                "completed_at": DateTime::now()
            }
        };
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn mark_task_failed(&self, task_id: &str) -> Result<(), MongoError> {
        let filter = doc! {
            "id": task_id
        };
        let update = doc! {
            "$set": {
                "status": "Failed"
            }
        };
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn get_all_tasks(&self) -> Result<Vec<TodoTask>, MongoError> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut tasks = Vec::new();
        while let Some(task) = cursor.try_next().await? {
            tasks.push(task);
        }
        Ok(tasks)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<TodoTask>, MongoError> {
        let filter = doc! {
            "id": task_id
        };
        Ok(self.collection.find_one(filter, None).await?)
    }

    pub async fn is_empty(&self) -> Result<bool, MongoError> {
        Ok(self.collection.count_documents(None, None).await? == 0)
    }

    pub async fn len(&self) -> Result<u64, MongoError> {
        Ok(self.collection.count_documents(None, None).await?)
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
            if let Some(task) = self.get_todo_list().get_next_task().await? {
                match self.process_task(task.clone()).await {
                    Ok(_) => {
                        self.get_todo_list().mark_task_completed(&task.id).await?;
                    }
                    Err(_) => {
                        self.get_todo_list().mark_task_failed(&task.id).await?;
                    }
                }
            }
            tokio::time::sleep(self.get_check_interval()).await;
        }
    }
}
