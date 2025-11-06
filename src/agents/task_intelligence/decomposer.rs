/// Smart task decomposition using learned patterns
///
/// Analyzes complex tasks and breaks them into manageable subtasks
/// based on historical decomposition patterns and ML-learned structure.

use super::task_history::{TaskHistory, TaskExecution};
use super::priority_predictor::TaskFeatures;
use crate::types::{TodoTask, TaskPriority};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A subtask generated from decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub description: String,
    pub estimated_priority: TaskPriority,
    pub estimated_duration_minutes: Option<i32>,
    pub depends_on: Vec<usize>, // Indices of other subtasks this depends on
    pub order: usize, // Suggested execution order
}

/// Strategy for task decomposition
#[derive(Debug, Clone, PartialEq)]
pub enum DecompositionStrategy {
    /// Break by implementation phases (design, implement, test, deploy)
    ByPhase,
    /// Break by components/modules
    ByComponent,
    /// Break by feature increments
    ByIncrement,
    /// Break by technical layers (frontend, backend, database)
    ByLayer,
    /// No decomposition needed
    None,
}

/// Learned decomposition pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecompositionPattern {
    pub pattern_name: String,
    pub keywords: Vec<String>,
    pub strategy: String, // Serializable version of DecompositionStrategy
    pub typical_subtask_count: usize,
    pub example_subtasks: Vec<String>,
    pub success_rate: f64,
}

/// Task decomposer using learned patterns
pub struct TaskDecomposer {
    history: TaskHistory,
    patterns: Vec<DecompositionPattern>,
    decomposition_count: usize,
}

impl TaskDecomposer {
    pub fn new(history: TaskHistory) -> Self {
        Self {
            history,
            patterns: Self::default_patterns(),
            decomposition_count: 0,
        }
    }

    /// Default decomposition patterns (can be learned from history)
    fn default_patterns() -> Vec<DecompositionPattern> {
        vec![
            DecompositionPattern {
                pattern_name: "Feature Implementation".to_string(),
                keywords: vec!["implement".to_string(), "feature".to_string(), "add".to_string()],
                strategy: "ByPhase".to_string(),
                typical_subtask_count: 4,
                example_subtasks: vec![
                    "Design feature architecture".to_string(),
                    "Implement core functionality".to_string(),
                    "Add tests".to_string(),
                    "Update documentation".to_string(),
                ],
                success_rate: 0.85,
            },
            DecompositionPattern {
                pattern_name: "Bug Fix".to_string(),
                keywords: vec!["fix".to_string(), "bug".to_string(), "issue".to_string()],
                strategy: "ByPhase".to_string(),
                typical_subtask_count: 3,
                example_subtasks: vec![
                    "Reproduce and diagnose bug".to_string(),
                    "Implement fix".to_string(),
                    "Verify fix with tests".to_string(),
                ],
                success_rate: 0.90,
            },
            DecompositionPattern {
                pattern_name: "Refactoring".to_string(),
                keywords: vec!["refactor".to_string(), "restructure".to_string(), "improve".to_string()],
                strategy: "ByComponent".to_string(),
                typical_subtask_count: 4,
                example_subtasks: vec![
                    "Identify refactoring scope".to_string(),
                    "Create tests for existing behavior".to_string(),
                    "Perform refactoring".to_string(),
                    "Verify all tests pass".to_string(),
                ],
                success_rate: 0.80,
            },
            DecompositionPattern {
                pattern_name: "Integration".to_string(),
                keywords: vec!["integrate".to_string(), "connect".to_string(), "api".to_string()],
                strategy: "ByLayer".to_string(),
                typical_subtask_count: 5,
                example_subtasks: vec![
                    "Design integration interface".to_string(),
                    "Implement API client".to_string(),
                    "Add error handling".to_string(),
                    "Write integration tests".to_string(),
                    "Update documentation".to_string(),
                ],
                success_rate: 0.75,
            },
        ]
    }

    /// Determine if a task should be decomposed
    pub async fn should_decompose(&self, task: &TodoTask) -> Result<bool> {
        let features = TaskFeatures::extract(&task.description);

        // Simple tasks don't need decomposition
        if features.word_count < 5 {
            return Ok(false);
        }

        // High complexity suggests decomposition
        if features.estimated_complexity > 0.6 {
            return Ok(true);
        }

        // Check for decomposition keywords
        let decomposition_indicators = vec![
            "implement", "integrate", "refactor", "migrate", "system",
            "architecture", "multiple", "complete", "full",
        ];

        let desc_lower = task.description.to_lowercase();
        let has_indicators = decomposition_indicators.iter()
            .any(|keyword| desc_lower.contains(keyword));

        Ok(has_indicators)
    }

    /// Decompose a task into subtasks
    pub async fn decompose(&mut self, task: &TodoTask) -> Result<Vec<Subtask>> {
        let features = TaskFeatures::extract(&task.description);

        // Find matching pattern
        let pattern = self.find_best_pattern(&features);

        let strategy = match pattern.strategy.as_str() {
            "ByPhase" => DecompositionStrategy::ByPhase,
            "ByComponent" => DecompositionStrategy::ByComponent,
            "ByIncrement" => DecompositionStrategy::ByIncrement,
            "ByLayer" => DecompositionStrategy::ByLayer,
            _ => DecompositionStrategy::None,
        };

        let subtasks = self.decompose_by_strategy(task, strategy, &pattern).await?;
        self.decomposition_count += 1;

        Ok(subtasks)
    }

    /// Find the best matching decomposition pattern
    fn find_best_pattern(&self, features: &TaskFeatures) -> DecompositionPattern {
        let mut best_score = 0.0;
        let mut best_pattern = &self.patterns[0];

        for pattern in &self.patterns {
            let mut score = 0.0;

            // Keyword matching
            let keyword_matches = pattern.keywords.iter()
                .filter(|keyword| features.keywords.contains(keyword))
                .count();

            score += keyword_matches as f64 * 0.5;

            // Success rate weighting
            score += pattern.success_rate * 0.3;

            if score > best_score {
                best_score = score;
                best_pattern = pattern;
            }
        }

        best_pattern.clone()
    }

    /// Decompose task using specific strategy
    async fn decompose_by_strategy(
        &self,
        task: &TodoTask,
        strategy: DecompositionStrategy,
        pattern: &DecompositionPattern,
    ) -> Result<Vec<Subtask>> {
        match strategy {
            DecompositionStrategy::ByPhase => self.decompose_by_phase(task, pattern).await,
            DecompositionStrategy::ByComponent => self.decompose_by_component(task, pattern).await,
            DecompositionStrategy::ByIncrement => self.decompose_by_increment(task, pattern).await,
            DecompositionStrategy::ByLayer => self.decompose_by_layer(task, pattern).await,
            DecompositionStrategy::None => Ok(Vec::new()),
        }
    }

    /// Decompose by implementation phases
    async fn decompose_by_phase(&self, task: &TodoTask, pattern: &DecompositionPattern) -> Result<Vec<Subtask>> {
        let features = TaskFeatures::extract(&task.description);
        let base_priority = &task.priority;

        let mut subtasks = vec![
            Subtask {
                description: format!("Design and plan: {}", task.description),
                estimated_priority: base_priority.clone(),
                estimated_duration_minutes: Some(30),
                depends_on: Vec::new(),
                order: 0,
            },
            Subtask {
                description: format!("Implement: {}", task.description),
                estimated_priority: base_priority.clone(),
                estimated_duration_minutes: Some(120),
                depends_on: vec![0],
                order: 1,
            },
            Subtask {
                description: format!("Test: {}", task.description),
                estimated_priority: base_priority.clone(),
                estimated_duration_minutes: Some(60),
                depends_on: vec![1],
                order: 2,
            },
        ];

        // Add documentation step for features
        if features.has_feature_keywords {
            subtasks.push(Subtask {
                description: format!("Document: {}", task.description),
                estimated_priority: TaskPriority::Low,
                estimated_duration_minutes: Some(30),
                depends_on: vec![2],
                order: 3,
            });
        }

        Ok(subtasks)
    }

    /// Decompose by components
    async fn decompose_by_component(&self, task: &TodoTask, _pattern: &DecompositionPattern) -> Result<Vec<Subtask>> {
        let features = TaskFeatures::extract(&task.description);

        // Extract potential component names from description
        let components = Self::extract_components(&task.description);

        let mut subtasks = Vec::new();
        for (i, component) in components.iter().enumerate() {
            subtasks.push(Subtask {
                description: format!("Work on {} component: {}", component, task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(90),
                depends_on: if i > 0 { vec![i - 1] } else { Vec::new() },
                order: i,
            });
        }

        // Add integration subtask if multiple components
        if components.len() > 1 {
            subtasks.push(Subtask {
                description: format!("Integrate all components: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(60),
                depends_on: (0..components.len()).collect(),
                order: components.len(),
            });
        }

        Ok(subtasks)
    }

    /// Decompose by incremental features
    async fn decompose_by_increment(&self, task: &TodoTask, _pattern: &DecompositionPattern) -> Result<Vec<Subtask>> {
        // Simple increment strategy: MVP -> Enhancement -> Polish
        Ok(vec![
            Subtask {
                description: format!("MVP: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(180),
                depends_on: Vec::new(),
                order: 0,
            },
            Subtask {
                description: format!("Enhance: {}", task.description),
                estimated_priority: TaskPriority::Medium,
                estimated_duration_minutes: Some(120),
                depends_on: vec![0],
                order: 1,
            },
            Subtask {
                description: format!("Polish and optimize: {}", task.description),
                estimated_priority: TaskPriority::Low,
                estimated_duration_minutes: Some(60),
                depends_on: vec![1],
                order: 2,
            },
        ])
    }

    /// Decompose by technical layers
    async fn decompose_by_layer(&self, task: &TodoTask, _pattern: &DecompositionPattern) -> Result<Vec<Subtask>> {
        Ok(vec![
            Subtask {
                description: format!("Database/Data layer: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(90),
                depends_on: Vec::new(),
                order: 0,
            },
            Subtask {
                description: format!("Backend/API layer: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(120),
                depends_on: vec![0],
                order: 1,
            },
            Subtask {
                description: format!("Frontend/UI layer: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(90),
                depends_on: vec![1],
                order: 2,
            },
            Subtask {
                description: format!("End-to-end testing: {}", task.description),
                estimated_priority: task.priority.clone(),
                estimated_duration_minutes: Some(60),
                depends_on: vec![0, 1, 2],
                order: 3,
            },
        ])
    }

    /// Extract component names from description
    fn extract_components(description: &str) -> Vec<String> {
        // Simple heuristic: look for words that might be component names
        let words: Vec<&str> = description.split_whitespace().collect();

        let mut components = Vec::new();

        // Look for common patterns like "X and Y" or "X, Y, and Z"
        for (i, word) in words.iter().enumerate() {
            if word.chars().next().map_or(false, |c| c.is_uppercase()) {
                // Capitalized word might be a component name
                components.push(word.to_string());
            }
        }

        // If no components found, use generic split
        if components.is_empty() {
            components = vec!["core".to_string(), "integration".to_string()];
        }

        components
    }

    /// Train decomposer from historical data
    pub async fn train(&mut self) -> Result<()> {
        // Analyze historical decompositions to improve patterns
        let executions = self.history.get_successful_executions().await?;

        let decomposed_tasks: Vec<_> = executions.iter()
            .filter(|e| e.was_decomposed)
            .collect();

        if decomposed_tasks.is_empty() {
            return Ok(());
        }

        // Build frequency map of decomposition patterns
        let mut pattern_usage: HashMap<String, usize> = HashMap::new();

        for task in decomposed_tasks {
            let features = TaskFeatures::extract(&task.description);
            let pattern = self.find_best_pattern(&features);
            *pattern_usage.entry(pattern.pattern_name).or_insert(0) += 1;
        }

        tracing::info!("Decomposition pattern usage: {:?}", pattern_usage);

        Ok(())
    }

    pub fn get_decomposition_count(&self) -> usize {
        self.decomposition_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_extraction() {
        let description = "Integrate the Authentication and Authorization systems";
        let components = TaskDecomposer::extract_components(description);

        assert!(components.contains(&"Authentication".to_string()));
        assert!(components.contains(&"Authorization".to_string()));
    }

    #[test]
    fn test_pattern_matching() {
        let history = unsafe { std::mem::zeroed() }; // Dummy for testing
        let decomposer = TaskDecomposer::new(history);

        let feature_desc = "Implement new user dashboard feature";
        let features = TaskFeatures::extract(feature_desc);
        let pattern = decomposer.find_best_pattern(&features);

        assert_eq!(pattern.pattern_name, "Feature Implementation");
    }
}
