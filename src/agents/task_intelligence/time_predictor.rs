/// Execution time prediction using historical data and ML
///
/// Predicts how long a task will take based on historical execution
/// patterns, task complexity, and learned agent performance characteristics.

use super::task_history::{TaskHistory, TaskExecution, TaskOutcome};
use super::priority_predictor::TaskFeatures;
use crate::types::{TodoTask, TaskPriority};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A duration prediction with confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationPrediction {
    pub estimated_seconds: i64,
    pub confidence: f64, // 0.0 to 1.0
    pub min_seconds: i64, // Lower bound (10th percentile)
    pub max_seconds: i64, // Upper bound (90th percentile)
    pub based_on_samples: usize,
}

impl DurationPrediction {
    pub fn estimated_minutes(&self) -> i32 {
        (self.estimated_seconds / 60) as i32
    }

    pub fn estimated_hours(&self) -> f64 {
        self.estimated_seconds as f64 / 3600.0
    }
}

/// Agent performance profile for time prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentProfile {
    pub agent_name: String,
    pub avg_task_duration: f64,
    pub std_deviation: f64,
    pub tasks_completed: usize,
    pub priority_durations: HashMap<String, f64>, // priority -> avg duration
}

/// Time predictor using historical data and ML
pub struct TimePredictor {
    history: TaskHistory,
    agent_profiles: HashMap<String, AgentProfile>,
}

impl TimePredictor {
    pub fn new(history: TaskHistory) -> Self {
        Self {
            history,
            agent_profiles: HashMap::new(),
        }
    }

    /// Predict execution time for a task
    pub async fn predict(&self, task: &TodoTask) -> Result<DurationPrediction> {
        let features = TaskFeatures::extract(&task.description);

        // Get similar historical tasks
        let similar_tasks = self.history.find_similar_tasks(&features.keywords).await?;

        if similar_tasks.is_empty() {
            // No history - use heuristics
            return Ok(self.heuristic_prediction(task, &features));
        }

        // Filter to successful tasks with duration data
        let valid_samples: Vec<_> = similar_tasks.iter()
            .filter(|e| e.outcome == TaskOutcome::Success && e.duration_seconds.is_some())
            .collect();

        if valid_samples.is_empty() {
            return Ok(self.heuristic_prediction(task, &features));
        }

        // Calculate statistics
        let durations: Vec<i64> = valid_samples.iter()
            .filter_map(|e| e.duration_seconds)
            .collect();

        let mean = durations.iter().sum::<i64>() as f64 / durations.len() as f64;
        let variance = durations.iter()
            .map(|&d| {
                let diff = d as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / durations.len() as f64;
        let std_dev = variance.sqrt();

        // Calculate percentiles
        let mut sorted_durations = durations.clone();
        sorted_durations.sort();

        let p10_idx = (durations.len() as f64 * 0.1) as usize;
        let p90_idx = (durations.len() as f64 * 0.9) as usize;

        let min_seconds = sorted_durations.get(p10_idx).copied().unwrap_or(durations[0]);
        let max_seconds = sorted_durations.get(p90_idx).copied().unwrap_or(*durations.last().unwrap());

        // Adjust for task complexity
        let complexity_factor = 1.0 + (features.estimated_complexity - 0.5) * 0.5;
        let adjusted_mean = mean * complexity_factor;

        // Adjust for priority (higher priority tasks often take longer due to thoroughness)
        let priority_factor = match task.priority {
            TaskPriority::Critical => 1.2,
            TaskPriority::High => 1.1,
            TaskPriority::Medium => 1.0,
            TaskPriority::Low => 0.9,
            TaskPriority::Inital => 1.0,
        };
        let final_estimate = (adjusted_mean * priority_factor) as i64;

        // Confidence based on sample size and variance
        let confidence = self.calculate_confidence(durations.len(), std_dev, mean);

        Ok(DurationPrediction {
            estimated_seconds: final_estimate,
            confidence,
            min_seconds,
            max_seconds,
            based_on_samples: durations.len(),
        })
    }

    /// Heuristic prediction when no historical data available
    fn heuristic_prediction(&self, task: &TodoTask, features: &TaskFeatures) -> DurationPrediction {
        // Base estimate on complexity and priority
        let complexity_minutes = (features.estimated_complexity * 120.0) as i64; // 0-120 minutes

        let priority_base = match task.priority {
            TaskPriority::Critical => 180, // 3 hours
            TaskPriority::High => 120,     // 2 hours
            TaskPriority::Medium => 60,    // 1 hour
            TaskPriority::Low => 30,       // 30 minutes
            TaskPriority::Inital => 45,    // 45 minutes
        };

        let estimated_minutes = complexity_minutes.max(priority_base);
        let estimated_seconds = estimated_minutes * 60;

        DurationPrediction {
            estimated_seconds,
            confidence: 0.5, // Low confidence for heuristics
            min_seconds: estimated_seconds / 2,
            max_seconds: estimated_seconds * 2,
            based_on_samples: 0,
        }
    }

    /// Calculate prediction confidence
    fn calculate_confidence(&self, sample_count: usize, std_dev: f64, mean: f64) -> f64 {
        // More samples = higher confidence
        let sample_confidence = (sample_count as f64 / 20.0).min(1.0);

        // Lower variance = higher confidence
        let variance_ratio = if mean > 0.0 {
            std_dev / mean
        } else {
            1.0
        };
        let variance_confidence = (1.0 - variance_ratio.min(1.0)).max(0.0);

        // Weighted average
        sample_confidence * 0.6 + variance_confidence * 0.4
    }

    /// Build agent performance profiles
    async fn build_agent_profiles(&mut self) -> Result<()> {
        let executions = self.history.get_successful_executions().await?;

        let mut agent_data: HashMap<String, Vec<i64>> = HashMap::new();
        let mut agent_priority_data: HashMap<String, HashMap<String, Vec<i64>>> = HashMap::new();

        for exec in &executions {
            if let Some(duration) = exec.duration_seconds {
                agent_data.entry(exec.target_agent.clone())
                    .or_insert_with(Vec::new)
                    .push(duration);

                let priority_str = format!("{:?}", exec.original_priority);
                agent_priority_data.entry(exec.target_agent.clone())
                    .or_insert_with(HashMap::new)
                    .entry(priority_str)
                    .or_insert_with(Vec::new)
                    .push(duration);
            }
        }

        for (agent_name, durations) in agent_data {
            let mean = durations.iter().sum::<i64>() as f64 / durations.len() as f64;
            let variance = durations.iter()
                .map(|&d| {
                    let diff = d as f64 - mean;
                    diff * diff
                })
                .sum::<f64>() / durations.len() as f64;
            let std_dev = variance.sqrt();

            let mut priority_durations = HashMap::new();
            if let Some(priority_data) = agent_priority_data.get(&agent_name) {
                for (priority, prio_durations) in priority_data {
                    let prio_mean = prio_durations.iter().sum::<i64>() as f64 / prio_durations.len() as f64;
                    priority_durations.insert(priority.clone(), prio_mean);
                }
            }

            self.agent_profiles.insert(agent_name.clone(), AgentProfile {
                agent_name: agent_name.clone(),
                avg_task_duration: mean,
                std_deviation: std_dev,
                tasks_completed: durations.len(),
                priority_durations,
            });
        }

        Ok(())
    }

    /// Get prediction accuracy from historical data
    pub async fn get_accuracy(&self) -> Result<f64> {
        let executions = self.history.get_successful_executions().await?;

        let predictions_with_actual: Vec<_> = executions.iter()
            .filter(|e| e.predicted_duration_seconds.is_some() && e.duration_seconds.is_some())
            .collect();

        if predictions_with_actual.is_empty() {
            return Ok(0.0);
        }

        let mut errors = Vec::new();

        for exec in predictions_with_actual {
            let predicted = exec.predicted_duration_seconds.unwrap() as f64;
            let actual = exec.duration_seconds.unwrap() as f64;

            // Calculate percentage error
            let error = ((predicted - actual).abs() / actual * 100.0).min(100.0);
            errors.push(error);
        }

        let mean_error = errors.iter().sum::<f64>() / errors.len() as f64;

        // Convert error to accuracy (100% - error%)
        let accuracy = (100.0 - mean_error) / 100.0;

        Ok(accuracy.max(0.0))
    }

    /// Train the time predictor from historical data
    pub async fn train(&mut self) -> Result<()> {
        // Build agent profiles
        self.build_agent_profiles().await?;

        tracing::info!("Built time prediction profiles for {} agents", self.agent_profiles.len());

        // Calculate prediction accuracy
        let accuracy = self.get_accuracy().await?;
        tracing::info!("Time prediction accuracy: {:.1}%", accuracy * 100.0);

        Ok(())
    }

    /// Get agent performance profile
    pub fn get_agent_profile(&self, agent_name: &str) -> Option<&AgentProfile> {
        self.agent_profiles.get(agent_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_prediction() {
        let history = unsafe { std::mem::zeroed() }; // Dummy for testing
        let predictor = TimePredictor::new(history);

        let task = TodoTask {
            id: "test".to_string(),
            description: "Simple task".to_string(),
            enhanced_description: None,
            priority: TaskPriority::Medium,
            project: None,
            source_agent: None,
            target_agent: "test".to_string(),
            status: crate::types::TaskStatus::Pending,
            created_at: 0,
            completed_at: None,
            due_date: None,
            duration_minutes: None,
            notes: None,
            ticket: None,
            last_modified: None,
        };

        let features = TaskFeatures::extract(&task.description);
        let prediction = predictor.heuristic_prediction(&task, &features);

        assert!(prediction.estimated_seconds > 0);
        assert!(prediction.min_seconds < prediction.estimated_seconds);
        assert!(prediction.max_seconds > prediction.estimated_seconds);
        assert!(prediction.confidence > 0.0 && prediction.confidence <= 1.0);
    }

    #[test]
    fn test_confidence_calculation() {
        let history = unsafe { std::mem::zeroed() }; // Dummy for testing
        let predictor = TimePredictor::new(history);

        // High sample count, low variance = high confidence
        let conf1 = predictor.calculate_confidence(50, 10.0, 100.0);

        // Low sample count, high variance = low confidence
        let conf2 = predictor.calculate_confidence(5, 50.0, 100.0);

        assert!(conf1 > conf2);
        assert!(conf1 <= 1.0 && conf1 >= 0.0);
        assert!(conf2 <= 1.0 && conf2 >= 0.0);
    }
}
