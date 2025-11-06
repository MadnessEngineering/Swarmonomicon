/// Advanced Task Intelligence System
///
/// This module provides ML-powered task management capabilities:
/// - Priority prediction based on historical patterns
/// - Smart task decomposition using learned patterns
/// - Dependency graph learning from execution history
/// - Execution time and success prediction
///
/// Similar to the agent learning system, this integrates seamlessly with
/// the existing TodoList/TodoProcessor infrastructure while remaining
/// fully backward compatible.

pub mod task_history;
pub mod priority_predictor;
pub mod decomposer;
pub mod dependency_learner;
pub mod time_predictor;
pub mod smart_todo;

pub use task_history::{TaskHistory, TaskExecution, TaskOutcome};
pub use priority_predictor::{PriorityPredictor, TaskFeatures};
pub use decomposer::{TaskDecomposer, Subtask, DecompositionStrategy};
pub use dependency_learner::{DependencyLearner, TaskDependency, DependencyGraph};
pub use time_predictor::{TimePredictor, DurationPrediction};
pub use smart_todo::SmartTodoList;

use crate::types::{TodoTask, TaskPriority, TaskStatus};
use anyhow::Result;
use mongodb::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for task intelligence features
#[derive(Debug, Clone)]
pub struct TaskIntelligenceConfig {
    pub enabled: bool,
    pub mongo_client: Option<Client>,
    pub enable_priority_prediction: bool,
    pub enable_decomposition: bool,
    pub enable_dependency_learning: bool,
    pub enable_time_prediction: bool,
    pub learning_threshold: usize, // Minimum tasks before ML activates
}

impl TaskIntelligenceConfig {
    pub fn new(mongo_client: Client) -> Self {
        Self {
            enabled: true,
            mongo_client: Some(mongo_client),
            enable_priority_prediction: true,
            enable_decomposition: true,
            enable_dependency_learning: true,
            enable_time_prediction: true,
            learning_threshold: 20, // Need at least 20 tasks for reliable ML
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            mongo_client: None,
            enable_priority_prediction: false,
            enable_decomposition: false,
            enable_dependency_learning: false,
            enable_time_prediction: false,
            learning_threshold: 20,
        }
    }
}

impl Default for TaskIntelligenceConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Statistics about task intelligence learning
#[derive(Debug, Clone)]
pub struct TaskIntelligenceStats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub avg_completion_time: f64,
    pub priority_accuracy: f64,
    pub decomposition_count: usize,
    pub dependency_rules: usize,
}

/// Main task intelligence service integrating all ML components
pub struct TaskIntelligenceService {
    config: TaskIntelligenceConfig,
    history: Option<TaskHistory>,
    priority_predictor: Option<Arc<RwLock<PriorityPredictor>>>,
    decomposer: Option<Arc<RwLock<TaskDecomposer>>>,
    dependency_learner: Option<Arc<RwLock<DependencyLearner>>>,
    time_predictor: Option<Arc<RwLock<TimePredictor>>>,
}

impl TaskIntelligenceService {
    pub async fn new(config: TaskIntelligenceConfig) -> Result<Self> {
        if !config.enabled {
            return Ok(Self {
                config,
                history: None,
                priority_predictor: None,
                decomposer: None,
                dependency_learner: None,
                time_predictor: None,
            });
        }

        let mongo_client = config.mongo_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("MongoDB client required when task intelligence is enabled"))?;

        // Initialize task history tracker
        let history = TaskHistory::new(mongo_client.clone()).await?;

        // Initialize ML components based on config
        let priority_predictor = if config.enable_priority_prediction {
            Some(Arc::new(RwLock::new(
                PriorityPredictor::new(history.clone())
            )))
        } else {
            None
        };

        let decomposer = if config.enable_decomposition {
            Some(Arc::new(RwLock::new(
                TaskDecomposer::new(history.clone())
            )))
        } else {
            None
        };

        let dependency_learner = if config.enable_dependency_learning {
            Some(Arc::new(RwLock::new(
                DependencyLearner::new(history.clone()).await?
            )))
        } else {
            None
        };

        let time_predictor = if config.enable_time_prediction {
            Some(Arc::new(RwLock::new(
                TimePredictor::new(history.clone())
            )))
        } else {
            None
        };

        Ok(Self {
            config,
            history: Some(history),
            priority_predictor,
            decomposer,
            dependency_learner,
            time_predictor,
        })
    }

    /// Record a task execution for learning
    pub async fn record_task_execution(
        &self,
        task: &TodoTask,
        outcome: TaskOutcome,
        actual_duration_seconds: i64,
    ) -> Result<()> {
        if let Some(history) = &self.history {
            history.record_execution(task, outcome, actual_duration_seconds).await?;
        }
        Ok(())
    }

    /// Predict optimal priority for a new task
    pub async fn predict_priority(&self, task_description: &str) -> Result<Option<TaskPriority>> {
        if let Some(predictor) = &self.priority_predictor {
            if let Some(history) = &self.history {
                let task_count = history.get_task_count().await?;
                if task_count >= self.config.learning_threshold {
                    let priority = predictor.read().await.predict(task_description).await?;
                    return Ok(Some(priority));
                }
            }
        }
        Ok(None)
    }

    /// Decompose a complex task into subtasks
    pub async fn decompose_task(&self, task: &TodoTask) -> Result<Option<Vec<Subtask>>> {
        if let Some(decomposer) = &self.decomposer {
            if let Some(history) = &self.history {
                let task_count = history.get_task_count().await?;
                if task_count >= self.config.learning_threshold {
                    let subtasks = decomposer.write().await.decompose(task).await?;
                    return Ok(Some(subtasks));
                }
            }
        }
        Ok(None)
    }

    /// Find dependencies for a task
    pub async fn find_dependencies(&self, task: &TodoTask) -> Result<Option<Vec<TaskDependency>>> {
        if let Some(learner) = &self.dependency_learner {
            if let Some(history) = &self.history {
                let task_count = history.get_task_count().await?;
                if task_count >= self.config.learning_threshold {
                    let deps = learner.read().await.find_dependencies(task).await?;
                    return Ok(Some(deps));
                }
            }
        }
        Ok(None)
    }

    /// Predict execution time for a task
    pub async fn predict_execution_time(&self, task: &TodoTask) -> Result<Option<DurationPrediction>> {
        if let Some(predictor) = &self.time_predictor {
            if let Some(history) = &self.history {
                let task_count = history.get_task_count().await?;
                if task_count >= self.config.learning_threshold {
                    let prediction = predictor.read().await.predict(task).await?;
                    return Ok(Some(prediction));
                }
            }
        }
        Ok(None)
    }

    /// Get statistics about task intelligence learning
    pub async fn get_stats(&self) -> Result<TaskIntelligenceStats> {
        if let Some(history) = &self.history {
            let total_tasks = history.get_task_count().await?;
            let successful_tasks = history.get_successful_task_count().await?;
            let failed_tasks = history.get_failed_task_count().await?;
            let avg_completion_time = history.get_avg_completion_time().await?;

            let priority_accuracy = if let Some(predictor) = &self.priority_predictor {
                predictor.read().await.get_accuracy().await?
            } else {
                0.0
            };

            let decomposition_count = if let Some(decomposer) = &self.decomposer {
                decomposer.read().await.get_decomposition_count()
            } else {
                0
            };

            let dependency_rules = if let Some(learner) = &self.dependency_learner {
                learner.read().await.get_rule_count().await?
            } else {
                0
            };

            Ok(TaskIntelligenceStats {
                total_tasks,
                successful_tasks,
                failed_tasks,
                avg_completion_time,
                priority_accuracy,
                decomposition_count,
                dependency_rules,
            })
        } else {
            Ok(TaskIntelligenceStats {
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

    /// Train all ML components from historical data
    pub async fn train_all(&self) -> Result<()> {
        if let Some(predictor) = &self.priority_predictor {
            predictor.write().await.train().await?;
        }
        if let Some(decomposer) = &self.decomposer {
            decomposer.write().await.train().await?;
        }
        if let Some(learner) = &self.dependency_learner {
            learner.write().await.train().await?;
        }
        if let Some(predictor) = &self.time_predictor {
            predictor.write().await.train().await?;
        }
        Ok(())
    }
}
