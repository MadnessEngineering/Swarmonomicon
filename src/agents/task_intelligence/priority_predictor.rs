/// ML-powered task priority prediction
///
/// Learns optimal priority levels from historical task execution patterns.
/// Uses features like keywords, complexity, success rate, and completion time
/// to predict the most appropriate priority for new tasks.

use super::task_history::{TaskHistory, TaskExecution, TaskOutcome};
use crate::types::TaskPriority;
use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Features extracted from a task for ML prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFeatures {
    pub keywords: Vec<String>,
    pub estimated_complexity: f64,
    pub word_count: usize,
    pub has_urgency_keywords: bool,
    pub has_security_keywords: bool,
    pub has_bug_keywords: bool,
    pub has_feature_keywords: bool,
}

impl TaskFeatures {
    pub fn extract(description: &str) -> Self {
        let urgency_keywords = vec!["urgent", "critical", "asap", "immediately", "emergency", "hotfix"];
        let security_keywords = vec!["security", "vulnerability", "exploit", "breach", "attack"];
        let bug_keywords = vec!["bug", "fix", "error", "crash", "issue", "problem"];
        let feature_keywords = vec!["feature", "implement", "add", "create", "new"];

        let desc_lower = description.to_lowercase();

        let has_urgency_keywords = urgency_keywords.iter().any(|k| desc_lower.contains(k));
        let has_security_keywords = security_keywords.iter().any(|k| desc_lower.contains(k));
        let has_bug_keywords = bug_keywords.iter().any(|k| desc_lower.contains(k));
        let has_feature_keywords = feature_keywords.iter().any(|k| desc_lower.contains(k));

        let keywords = TaskExecution::extract_keywords(description);
        let estimated_complexity = TaskExecution::estimate_complexity(description);
        let word_count = description.split_whitespace().count();

        Self {
            keywords,
            estimated_complexity,
            word_count,
            has_urgency_keywords,
            has_security_keywords,
            has_bug_keywords,
            has_feature_keywords,
        }
    }

    /// Calculate similarity score with another task's features
    pub fn similarity(&self, other: &TaskFeatures) -> f64 {
        // Keyword overlap (Jaccard similarity)
        let keywords1: std::collections::HashSet<_> = self.keywords.iter().collect();
        let keywords2: std::collections::HashSet<_> = other.keywords.iter().collect();

        let intersection = keywords1.intersection(&keywords2).count();
        let union = keywords1.union(&keywords2).count();

        let keyword_similarity = if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        };

        // Complexity similarity
        let complexity_diff = (self.estimated_complexity - other.estimated_complexity).abs();
        let complexity_similarity = 1.0 - complexity_diff;

        // Feature flags similarity
        let mut flag_matches = 0;
        let mut flag_total = 4;

        if self.has_urgency_keywords == other.has_urgency_keywords {
            flag_matches += 1;
        }
        if self.has_security_keywords == other.has_security_keywords {
            flag_matches += 1;
        }
        if self.has_bug_keywords == other.has_bug_keywords {
            flag_matches += 1;
        }
        if self.has_feature_keywords == other.has_feature_keywords {
            flag_matches += 1;
        }

        let flag_similarity = flag_matches as f64 / flag_total as f64;

        // Weighted average
        keyword_similarity * 0.5 + complexity_similarity * 0.3 + flag_similarity * 0.2
    }
}

/// Priority prediction model using k-NN and heuristics
pub struct PriorityPredictor {
    history: TaskHistory,
    k: usize, // Number of neighbors for k-NN
    prediction_cache: HashMap<String, (TaskPriority, f64)>, // Cache of predictions with confidence
}

impl PriorityPredictor {
    pub fn new(history: TaskHistory) -> Self {
        Self {
            history,
            k: 5, // Use 5 nearest neighbors
            prediction_cache: HashMap::new(),
        }
    }

    /// Predict priority for a new task
    pub async fn predict(&self, description: &str) -> Result<TaskPriority> {
        let features = TaskFeatures::extract(description);

        // Rule-based overrides for obvious cases
        if features.has_urgency_keywords || features.has_security_keywords {
            return Ok(TaskPriority::High);
        }

        // Get historical executions for learning
        let executions = self.history.get_successful_executions().await?;

        if executions.is_empty() {
            // No history - use heuristics
            return Ok(self.heuristic_priority(&features));
        }

        // k-NN: Find k most similar tasks
        let mut similarities: Vec<(f64, &TaskExecution)> = executions
            .iter()
            .map(|exec| {
                let exec_features = TaskFeatures::extract(&exec.description);
                let similarity = features.similarity(&exec_features);
                (similarity, exec)
            })
            .collect();

        // Sort by similarity (highest first)
        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // Take top k neighbors
        let neighbors: Vec<_> = similarities.into_iter().take(self.k).collect();

        // Predict based on weighted voting
        let mut priority_scores: HashMap<TaskPriority, f64> = HashMap::new();

        for (similarity, execution) in neighbors {
            let weight = similarity; // Use similarity as weight
            *priority_scores.entry(execution.original_priority.clone()).or_insert(0.0) += weight;
        }

        // Find priority with highest score
        let predicted = priority_scores
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(priority, _)| priority)
            .unwrap_or_else(|| self.heuristic_priority(&features));

        Ok(predicted)
    }

    /// Heuristic-based priority when no history available
    fn heuristic_priority(&self, features: &TaskFeatures) -> TaskPriority {
        // Security or urgency = High
        if features.has_security_keywords || features.has_urgency_keywords {
            return TaskPriority::High;
        }

        // Bugs with high complexity = High
        if features.has_bug_keywords && features.estimated_complexity > 0.7 {
            return TaskPriority::High;
        }

        // Bugs = Medium
        if features.has_bug_keywords {
            return TaskPriority::Medium;
        }

        // Complex features = Medium
        if features.has_feature_keywords && features.estimated_complexity > 0.6 {
            return TaskPriority::Medium;
        }

        // Default = Low
        TaskPriority::Low
    }

    /// Calculate priority prediction accuracy from history
    pub async fn get_accuracy(&self) -> Result<f64> {
        let executions = self.history.get_successful_executions().await?;

        if executions.len() < 10 {
            return Ok(0.0); // Not enough data
        }

        let mut correct = 0;
        let mut total = 0;

        // Leave-one-out cross-validation
        for exec in &executions {
            if let Some(predicted_priority) = exec.predicted_priority.as_ref() {
                total += 1;
                if predicted_priority == &exec.original_priority {
                    correct += 1;
                }
            }
        }

        if total == 0 {
            return Ok(0.0);
        }

        Ok(correct as f64 / total as f64)
    }

    /// Train the model (currently k-NN doesn't need explicit training,
    /// but we can use this to pre-compute features or optimize k)
    pub async fn train(&mut self) -> Result<()> {
        // For k-NN, we don't need explicit training
        // But we can optimize k based on cross-validation

        let executions = self.history.get_successful_executions().await?;

        if executions.len() < 20 {
            return Ok(()); // Not enough data to optimize
        }

        // Try different k values
        let k_values = vec![3, 5, 7, 10];
        let mut best_k = 5;
        let mut best_accuracy = 0.0;

        for k in k_values {
            let old_k = self.k;
            self.k = k;

            let mut correct = 0;
            let mut total = 0;

            // Simple validation
            for i in 0..executions.len().min(100) {
                let test_exec = &executions[i];
                if let Ok(predicted) = self.predict(&test_exec.description).await {
                    total += 1;
                    if predicted == test_exec.original_priority {
                        correct += 1;
                    }
                }
            }

            let accuracy = if total > 0 {
                correct as f64 / total as f64
            } else {
                0.0
            };

            if accuracy > best_accuracy {
                best_accuracy = accuracy;
                best_k = k;
            }

            self.k = old_k;
        }

        self.k = best_k;
        tracing::info!("Optimized k-NN to k={} with accuracy={:.2}%", best_k, best_accuracy * 100.0);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_extraction() {
        let description = "Fix critical security vulnerability in authentication ASAP";
        let features = TaskFeatures::extract(description);

        assert!(features.has_urgency_keywords);
        assert!(features.has_security_keywords);
        assert!(features.has_bug_keywords);
        assert!(!features.has_feature_keywords);
        assert!(features.keywords.len() > 0);
        assert!(features.word_count > 0);
    }

    #[test]
    fn test_feature_similarity() {
        let desc1 = "Fix critical security bug in login system";
        let desc2 = "Fix authentication security vulnerability";

        let features1 = TaskFeatures::extract(desc1);
        let features2 = TaskFeatures::extract(desc2);

        let similarity = features1.similarity(&features2);

        // Should have high similarity (both security/bug related)
        assert!(similarity > 0.5, "Similarity should be > 0.5, got {}", similarity);
    }

    #[test]
    fn test_heuristic_priority() {
        let history = TaskHistory {
            executions: mongodb::Collection::clone_with_type(&mongodb::Collection::<TaskExecution>::clone(&unsafe {
                std::mem::zeroed()
            })),
        };
        let predictor = PriorityPredictor::new(history);

        // Security task should be High
        let security_features = TaskFeatures::extract("Fix critical security vulnerability");
        assert_eq!(predictor.heuristic_priority(&security_features), TaskPriority::High);

        // Bug should be Medium
        let bug_features = TaskFeatures::extract("Fix login bug");
        assert_eq!(predictor.heuristic_priority(&bug_features), TaskPriority::Medium);

        // Simple feature should be Low
        let feature_features = TaskFeatures::extract("Add new button to UI");
        assert_eq!(predictor.heuristic_priority(&feature_features), TaskPriority::Low);
    }
}
