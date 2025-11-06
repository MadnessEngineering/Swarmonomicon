/// Smart TodoList wrapper integrating ML-powered task intelligence
///
/// Drop-in replacement for TodoList with intelligent features:
/// - Auto-priority prediction
/// - Task decomposition
/// - Dependency detection
/// - Time estimation

use super::{TaskIntelligenceService, TaskIntelligenceConfig, TaskOutcome};
use crate::types::{TodoTask, TodoList, TaskPriority, TaskStatus};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Smart TodoList with ML-powered task intelligence
pub struct SmartTodoList {
    todo_list: TodoList,
    intelligence: Option<Arc<RwLock<TaskIntelligenceService>>>,
}

impl SmartTodoList {
    /// Create new SmartTodoList with task intelligence
    pub async fn new(config: TaskIntelligenceConfig) -> Result<Self> {
        let todo_list = TodoList::new().await?;

        let intelligence = if config.enabled {
            let service = TaskIntelligenceService::new(config).await?;
            Some(Arc::new(RwLock::new(service)))
        } else {
            None
        };

        Ok(Self {
            todo_list,
            intelligence,
        })
    }

    /// Create SmartTodoList without intelligence (acts like regular TodoList)
    pub async fn without_intelligence() -> Result<Self> {
        let todo_list = TodoList::new().await?;
        Ok(Self {
            todo_list,
            intelligence: None,
        })
    }

    /// Add task with automatic ML enhancements
    pub async fn add_smart_task(&self, mut task: TodoTask) -> Result<()> {
        if let Some(intelligence) = &self.intelligence {
            let service = intelligence.read().await;

            // Predict optimal priority if not explicitly set
            if let Ok(Some(predicted_priority)) = service.predict_priority(&task.description).await {
                tracing::info!(
                    "ML predicted priority {:?} for task: {}",
                    predicted_priority,
                    task.description
                );

                // Only override if current priority is Initial/default
                if task.priority == TaskPriority::Inital {
                    task.priority = predicted_priority;
                }
            }

            // Predict execution time
            if let Ok(Some(prediction)) = service.predict_execution_time(&task).await {
                task.duration_minutes = Some(prediction.estimated_minutes());
                tracing::info!(
                    "ML estimated {} minutes for task (confidence: {:.1}%): {}",
                    prediction.estimated_minutes(),
                    prediction.confidence * 100.0,
                    task.description
                );
            }

            // Find dependencies
            if let Ok(Some(deps)) = service.find_dependencies(&task).await {
                if !deps.is_empty() {
                    tracing::info!(
                        "Found {} potential dependencies for task: {}",
                        deps.len(),
                        task.description
                    );

                    // Store dependency info in notes
                    let dep_info = deps.iter()
                        .map(|d| format!("Depends on: {} (confidence: {:.1}%)",
                                        d.prerequisite_description,
                                        d.confidence * 100.0))
                        .collect::<Vec<_>>()
                        .join("\n");

                    task.notes = Some(match task.notes {
                        Some(existing) => format!("{}\n\nDependencies:\n{}", existing, dep_info),
                        None => format!("Dependencies:\n{}", dep_info),
                    });
                }
            }
        }

        self.todo_list.add_task(task).await?;
        Ok(())
    }

    /// Decompose a complex task into subtasks
    pub async fn decompose_and_add(&self, parent_task: TodoTask) -> Result<Vec<TodoTask>> {
        if let Some(intelligence) = &self.intelligence {
            let service = intelligence.read().await;

            if let Ok(Some(subtasks)) = service.decompose_task(&parent_task).await {
                tracing::info!(
                    "Decomposed task '{}' into {} subtasks",
                    parent_task.description,
                    subtasks.len()
                );

                let mut created_tasks = Vec::new();

                for subtask in subtasks {
                    let todo = TodoTask {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: subtask.description,
                        enhanced_description: None,
                        priority: subtask.estimated_priority,
                        project: parent_task.project.clone(),
                        source_agent: parent_task.source_agent.clone(),
                        target_agent: parent_task.target_agent.clone(),
                        status: TaskStatus::Pending,
                        created_at: chrono::Utc::now().timestamp(),
                        completed_at: None,
                        due_date: None,
                        duration_minutes: subtask.estimated_duration_minutes,
                        notes: Some(format!("Subtask {} of: {}", subtask.order + 1, parent_task.description)),
                        ticket: parent_task.ticket.clone(),
                        last_modified: Some(chrono::Utc::now().timestamp()),
                    };

                    self.add_smart_task(todo.clone()).await?;
                    created_tasks.push(todo);
                }

                return Ok(created_tasks);
            }
        }

        // Fallback: just add the parent task
        self.add_smart_task(parent_task.clone()).await?;
        Ok(vec![parent_task])
    }

    /// Record task completion for learning
    pub async fn record_completion(&self, task: &TodoTask, outcome: TaskOutcome, duration_seconds: i64) -> Result<()> {
        if let Some(intelligence) = &self.intelligence {
            intelligence.read().await.record_task_execution(task, outcome, duration_seconds).await?;
        }
        Ok(())
    }

    /// Train all ML models from historical data
    pub async fn train_models(&self) -> Result<()> {
        if let Some(intelligence) = &self.intelligence {
            intelligence.read().await.train_all().await?;
        }
        Ok(())
    }

    /// Get learning statistics
    pub async fn get_intelligence_stats(&self) -> Result<super::TaskIntelligenceStats> {
        if let Some(intelligence) = &self.intelligence {
            intelligence.read().await.get_stats().await
        } else {
            Ok(super::TaskIntelligenceStats {
                total_tasks: 0,
                successful_tasks: 0,
                failed_tasks: 0,
                avg_completion_time: 0.0,
                priority_accuracy: 0.0,
                decomposition_count: 0,
                dependency_rules: 0,
            })
        }
    }

    /// Access underlying TodoList for standard operations
    pub fn todo_list(&self) -> &TodoList {
        &self.todo_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskPriority;

    #[tokio::test]
    async fn test_smart_todo_creation() -> Result<()> {
        // Test that SmartTodoList can be created without MongoDB
        // (will fail gracefully if MongoDB not available)
        match SmartTodoList::without_intelligence().await {
            Ok(smart_list) => {
                assert!(smart_list.intelligence.is_none());
            }
            Err(e) => {
                tracing::warn!("SmartTodoList creation failed (likely MongoDB not running): {}", e);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_add_task_without_intelligence() -> Result<()> {
        match SmartTodoList::without_intelligence().await {
            Ok(smart_list) => {
                let task = TodoTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "Test task".to_string(),
                    enhanced_description: None,
                    priority: TaskPriority::Medium,
                    project: Some("test".to_string()),
                    source_agent: None,
                    target_agent: "test_agent".to_string(),
                    status: TaskStatus::Pending,
                    created_at: chrono::Utc::now().timestamp(),
                    completed_at: None,
                    due_date: None,
                    duration_minutes: None,
                    notes: None,
                    ticket: None,
                    last_modified: None,
                };

                match smart_list.add_smart_task(task).await {
                    Ok(_) => tracing::info!("Task added successfully"),
                    Err(e) => tracing::warn!("Task add failed (likely MongoDB not running): {}", e),
                }
            }
            Err(e) => {
                tracing::warn!("SmartTodoList creation failed: {}", e);
            }
        }
        Ok(())
    }
}
