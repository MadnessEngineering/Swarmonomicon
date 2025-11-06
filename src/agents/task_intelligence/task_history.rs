/// Task execution history tracking for ML learning
///
/// Similar to InteractionTracker in the learning system, TaskHistory
/// records all task executions to MongoDB for analysis and learning.

use crate::types::{TodoTask, TaskPriority, TaskStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use futures_util::TryStreamExt;

/// Outcome of a task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskOutcome {
    Success,
    Failure,
    Timeout,
    Cancelled,
}

/// A record of a task execution for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: String,
    pub task_id: String,
    pub description: String,
    pub enhanced_description: Option<String>,
    pub original_priority: TaskPriority,
    pub predicted_priority: Option<TaskPriority>,
    pub project: Option<String>,
    pub target_agent: String,
    pub source_agent: Option<String>,

    // Execution details
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub predicted_duration_seconds: Option<i64>,
    pub outcome: TaskOutcome,

    // Decomposition tracking
    pub was_decomposed: bool,
    pub parent_task_id: Option<String>,
    pub subtask_ids: Vec<String>,

    // Dependency tracking
    pub dependencies: Vec<String>, // IDs of tasks this depended on
    pub blocked_tasks: Vec<String>, // IDs of tasks blocked by this

    // ML features extracted from description
    pub keywords: Vec<String>,
    pub estimated_complexity: f64, // 0.0 to 1.0
    pub category: Option<String>,

    // Metadata
    pub metadata: HashMap<String, String>,
}

impl TaskExecution {
    pub fn from_task(task: &TodoTask, outcome: TaskOutcome, duration_seconds: i64) -> Self {
        let completed_at = Some(Utc::now());
        let started_at = if let Some(created) = task.created_at.checked_sub(duration_seconds) {
            DateTime::from_timestamp(created, 0).unwrap_or_else(|| Utc::now())
        } else {
            DateTime::from_timestamp(task.created_at, 0).unwrap_or_else(|| Utc::now())
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            description: task.description.clone(),
            enhanced_description: task.enhanced_description.clone(),
            original_priority: task.priority.clone(),
            predicted_priority: None,
            project: task.project.clone(),
            target_agent: task.target_agent.clone(),
            source_agent: task.source_agent.clone(),
            started_at,
            completed_at,
            duration_seconds: Some(duration_seconds),
            predicted_duration_seconds: None,
            outcome,
            was_decomposed: false,
            parent_task_id: None,
            subtask_ids: Vec::new(),
            dependencies: Vec::new(),
            blocked_tasks: Vec::new(),
            keywords: Self::extract_keywords(&task.description),
            estimated_complexity: Self::estimate_complexity(&task.description),
            category: None,
            metadata: HashMap::new(),
        }
    }

    /// Extract keywords from task description for ML features
    pub fn extract_keywords(description: &str) -> Vec<String> {
        // Simple keyword extraction - can be enhanced with NLP
        let stopwords = vec!["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for"];

        description
            .to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 3 && !stopwords.contains(&word))
            .map(|s| s.to_string())
            .collect()
    }

    /// Estimate task complexity based on description (0.0 to 1.0)
    pub fn estimate_complexity(description: &str) -> f64 {
        // Simple heuristic - can be enhanced with ML
        let complexity_indicators = vec![
            "complex", "difficult", "challenging", "multiple", "integrate",
            "system", "architecture", "refactor", "migrate", "implement",
        ];

        let desc_lower = description.to_lowercase();
        let matches = complexity_indicators.iter()
            .filter(|&word| desc_lower.contains(word))
            .count();

        let word_count = description.split_whitespace().count();
        let length_factor = (word_count as f64 / 50.0).min(1.0); // More words = more complex
        let keyword_factor = (matches as f64 / 3.0).min(1.0); // More complexity keywords

        (length_factor * 0.5 + keyword_factor * 0.5).min(1.0)
    }
}

/// MongoDB-backed task execution history
#[derive(Clone)]
pub struct TaskHistory {
    executions: Collection<TaskExecution>,
}

impl TaskHistory {
    pub async fn new(mongo_client: Client) -> Result<Self> {
        let db = mongo_client.database("swarmonomicon");
        let executions = db.collection("task_executions");

        // Create indexes for efficient querying
        use mongodb::IndexModel;
        use mongodb::bson::doc;

        let indexes = vec![
            IndexModel::builder()
                .keys(doc! { "task_id": 1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "target_agent": 1, "started_at": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "outcome": 1, "started_at": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "project": 1, "started_at": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "original_priority": 1, "outcome": 1 })
                .build(),
        ];

        executions.create_indexes(indexes, None).await?;

        Ok(Self { executions })
    }

    /// Record a task execution
    pub async fn record_execution(
        &self,
        task: &TodoTask,
        outcome: TaskOutcome,
        duration_seconds: i64,
    ) -> Result<()> {
        let execution = TaskExecution::from_task(task, outcome, duration_seconds);
        self.executions.insert_one(execution, None).await?;
        Ok(())
    }

    /// Get all executions for a specific agent
    pub async fn get_agent_executions(&self, agent_name: &str) -> Result<Vec<TaskExecution>> {
        use mongodb::bson::doc;

        let filter = doc! { "target_agent": agent_name };
        let mut cursor = self.executions.find(filter, None).await?;

        let mut executions = Vec::new();
        while let Some(execution) = cursor.try_next().await? {
            executions.push(execution);
        }

        Ok(executions)
    }

    /// Get executions by outcome
    pub async fn get_executions_by_outcome(&self, outcome: TaskOutcome) -> Result<Vec<TaskExecution>> {
        use mongodb::bson::{doc, to_bson};

        let filter = doc! { "outcome": to_bson(&outcome)? };
        let mut cursor = self.executions.find(filter, None).await?;

        let mut executions = Vec::new();
        while let Some(execution) = cursor.try_next().await? {
            executions.push(execution);
        }

        Ok(executions)
    }

    /// Get all successful executions
    pub async fn get_successful_executions(&self) -> Result<Vec<TaskExecution>> {
        self.get_executions_by_outcome(TaskOutcome::Success).await
    }

    /// Get total task count
    pub async fn get_task_count(&self) -> Result<usize> {
        let count = self.executions.count_documents(None, None).await?;
        Ok(count as usize)
    }

    /// Get successful task count
    pub async fn get_successful_task_count(&self) -> Result<usize> {
        use mongodb::bson::{doc, to_bson};

        let filter = doc! { "outcome": to_bson(&TaskOutcome::Success)? };
        let count = self.executions.count_documents(filter, None).await?;
        Ok(count as usize)
    }

    /// Get failed task count
    pub async fn get_failed_task_count(&self) -> Result<usize> {
        use mongodb::bson::{doc, to_bson};

        let filter = doc! { "outcome": to_bson(&TaskOutcome::Failure)? };
        let count = self.executions.count_documents(filter, None).await?;
        Ok(count as usize)
    }

    /// Get average completion time for successful tasks
    pub async fn get_avg_completion_time(&self) -> Result<f64> {
        use mongodb::bson::{doc, to_bson, Document};

        let pipeline = vec![
            doc! {
                "$match": {
                    "outcome": to_bson(&TaskOutcome::Success)?,
                    "duration_seconds": { "$exists": true }
                }
            },
            doc! {
                "$group": {
                    "_id": null,
                    "avg_duration": { "$avg": "$duration_seconds" }
                }
            },
        ];

        let mut cursor = self.executions.aggregate(pipeline, None).await?;

        if let Some(result) = cursor.try_next().await? {
            if let Some(avg) = result.get("avg_duration").and_then(|v| v.as_f64()) {
                return Ok(avg);
            }
        }

        Ok(0.0)
    }

    /// Get executions for a specific project
    pub async fn get_project_executions(&self, project: &str) -> Result<Vec<TaskExecution>> {
        use mongodb::bson::doc;

        let filter = doc! { "project": project };
        let mut cursor = self.executions.find(filter, None).await?;

        let mut executions = Vec::new();
        while let Some(execution) = cursor.try_next().await? {
            executions.push(execution);
        }

        Ok(executions)
    }

    /// Get recent executions (last N)
    pub async fn get_recent_executions(&self, limit: i64) -> Result<Vec<TaskExecution>> {
        use mongodb::bson::doc;
        use mongodb::options::FindOptions;

        let options = FindOptions::builder()
            .sort(doc! { "started_at": -1 })
            .limit(limit)
            .build();

        let mut cursor = self.executions.find(None, options).await?;

        let mut executions = Vec::new();
        while let Some(execution) = cursor.try_next().await? {
            executions.push(execution);
        }

        Ok(executions)
    }

    /// Find similar tasks by keywords
    pub async fn find_similar_tasks(&self, keywords: &[String]) -> Result<Vec<TaskExecution>> {
        use mongodb::bson::doc;

        // MongoDB text search or array intersection
        let filter = doc! {
            "keywords": {
                "$in": keywords
            }
        };

        let mut cursor = self.executions.find(filter, None).await?;

        let mut executions = Vec::new();
        while let Some(execution) = cursor.try_next().await? {
            executions.push(execution);
        }

        Ok(executions)
    }

    /// Get priority distribution
    pub async fn get_priority_distribution(&self) -> Result<HashMap<TaskPriority, usize>> {
        use mongodb::bson::{doc, Document};

        let pipeline = vec![
            doc! {
                "$group": {
                    "_id": "$original_priority",
                    "count": { "$sum": 1 }
                }
            },
        ];

        let mut cursor = self.executions.aggregate(pipeline, None).await?;
        let mut distribution = HashMap::new();

        while let Some(result) = cursor.try_next().await? {
            if let (Some(priority_bson), Some(count)) = (
                result.get("_id"),
                result.get("count").and_then(|v| v.as_i32())
            ) {
                if let Ok(priority) = mongodb::bson::from_bson::<TaskPriority>(priority_bson.clone()) {
                    distribution.insert(priority, count as usize);
                }
            }
        }

        Ok(distribution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_extraction() {
        let description = "Fix the critical security vulnerability in the authentication system";
        let keywords = TaskExecution::extract_keywords(description);

        assert!(keywords.contains(&"critical".to_string()));
        assert!(keywords.contains(&"security".to_string()));
        assert!(keywords.contains(&"vulnerability".to_string()));
        assert!(keywords.contains(&"authentication".to_string()));
        assert!(keywords.contains(&"system".to_string()));

        // Stopwords should be filtered
        assert!(!keywords.contains(&"the".to_string()));
    }

    #[test]
    fn test_complexity_estimation() {
        let simple_task = "Update README";
        let complex_task = "Implement complex multi-system architecture refactor with challenging integration requirements";

        let simple_complexity = TaskExecution::estimate_complexity(simple_task);
        let complex_complexity = TaskExecution::estimate_complexity(complex_task);

        assert!(simple_complexity < complex_complexity);
        assert!(simple_complexity >= 0.0 && simple_complexity <= 1.0);
        assert!(complex_complexity >= 0.0 && complex_complexity <= 1.0);
    }
}
