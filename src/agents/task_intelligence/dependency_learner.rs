/// Dependency graph learning from task execution history
///
/// Learns which tasks typically depend on others by analyzing
/// execution order, timing, and relationships in historical data.

use super::task_history::{TaskHistory, TaskExecution, TaskOutcome};
use super::priority_predictor::TaskFeatures;
use crate::types::TodoTask;
use anyhow::Result;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A learned dependency between tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub prerequisite_description: String,
    pub prerequisite_keywords: Vec<String>,
    pub confidence: f64, // 0.0 to 1.0
    pub typical_gap_hours: f64, // Typical time between prerequisite and dependent task
}

/// Dependency rule learned from patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRule {
    pub name: String,
    pub prerequisite_keywords: Vec<String>,
    pub dependent_keywords: Vec<String>,
    pub confidence: f64,
    pub occurrence_count: usize,
}

/// A graph of task dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<String>, // Task IDs
    pub edges: HashMap<String, Vec<String>>, // task_id -> [dependent_task_ids]
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, task_id: String) {
        if !self.nodes.contains(&task_id) {
            self.nodes.push(task_id);
        }
    }

    pub fn add_edge(&mut self, from: String, to: String) {
        self.add_node(from.clone());
        self.add_node(to.clone());
        self.edges.entry(from).or_insert_with(Vec::new).push(to);
    }

    /// Get topological sort (execution order) of tasks
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for node in &self.nodes {
            in_degree.insert(node.clone(), 0);
        }
        for (_from, tos) in &self.edges {
            for to in tos {
                *in_degree.get_mut(to).unwrap() += 1;
            }
        }

        // Find nodes with no incoming edges
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(node, _)| node.clone())
            .collect();

        // Process queue
        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(neighbors) = self.edges.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.nodes.len() {
            return Err(anyhow::anyhow!("Dependency graph contains cycles"));
        }

        Ok(result)
    }
}

/// Learns task dependencies from execution history
pub struct DependencyLearner {
    history: TaskHistory,
    rules: Vec<DependencyRule>,
    dependency_cache: HashMap<String, Vec<TaskDependency>>,
}

impl DependencyLearner {
    pub async fn new(history: TaskHistory) -> Result<Self> {
        let mut learner = Self {
            history,
            rules: Vec::new(),
            dependency_cache: HashMap::new(),
        };

        // Initialize with default rules
        learner.rules = Self::default_rules();

        Ok(learner)
    }

    /// Default dependency rules
    fn default_rules() -> Vec<DependencyRule> {
        vec![
            DependencyRule {
                name: "Design before Implementation".to_string(),
                prerequisite_keywords: vec!["design".to_string(), "plan".to_string(), "architecture".to_string()],
                dependent_keywords: vec!["implement".to_string(), "code".to_string(), "build".to_string()],
                confidence: 0.9,
                occurrence_count: 0,
            },
            DependencyRule {
                name: "Implementation before Testing".to_string(),
                prerequisite_keywords: vec!["implement".to_string(), "code".to_string(), "build".to_string()],
                dependent_keywords: vec!["test".to_string(), "verify".to_string(), "validate".to_string()],
                confidence: 0.95,
                occurrence_count: 0,
            },
            DependencyRule {
                name: "Testing before Deployment".to_string(),
                prerequisite_keywords: vec!["test".to_string(), "verify".to_string()],
                dependent_keywords: vec!["deploy".to_string(), "release".to_string(), "publish".to_string()],
                confidence: 0.95,
                occurrence_count: 0,
            },
            DependencyRule {
                name: "Fix Bug before Feature".to_string(),
                prerequisite_keywords: vec!["fix".to_string(), "bug".to_string(), "issue".to_string()],
                dependent_keywords: vec!["feature".to_string(), "implement".to_string(), "add".to_string()],
                confidence: 0.7,
                occurrence_count: 0,
            },
            DependencyRule {
                name: "API before Frontend".to_string(),
                prerequisite_keywords: vec!["api".to_string(), "backend".to_string(), "endpoint".to_string()],
                dependent_keywords: vec!["frontend".to_string(), "ui".to_string(), "interface".to_string()],
                confidence: 0.85,
                occurrence_count: 0,
            },
            DependencyRule {
                name: "Database before API".to_string(),
                prerequisite_keywords: vec!["database".to_string(), "schema".to_string(), "model".to_string()],
                dependent_keywords: vec!["api".to_string(), "endpoint".to_string(), "service".to_string()],
                confidence: 0.8,
                occurrence_count: 0,
            },
        ]
    }

    /// Find dependencies for a task
    pub async fn find_dependencies(&self, task: &TodoTask) -> Result<Vec<TaskDependency>> {
        let features = TaskFeatures::extract(&task.description);
        let mut dependencies = Vec::new();

        // Check each rule
        for rule in &self.rules {
            // Check if task matches the dependent keywords
            let matches_dependent = rule.dependent_keywords.iter()
                .any(|keyword| features.keywords.contains(keyword));

            if matches_dependent {
                // This task might depend on something matching prerequisite keywords
                dependencies.push(TaskDependency {
                    prerequisite_description: format!(
                        "Task involving: {}",
                        rule.prerequisite_keywords.join(", ")
                    ),
                    prerequisite_keywords: rule.prerequisite_keywords.clone(),
                    confidence: rule.confidence,
                    typical_gap_hours: 24.0, // Default 1 day
                });
            }
        }

        // Find similar tasks from history and their predecessors
        let similar_tasks = self.history.find_similar_tasks(&features.keywords).await?;

        for similar in similar_tasks.iter().take(5) {
            if !similar.dependencies.is_empty() {
                // Get the dependency tasks
                for dep_id in &similar.dependencies {
                    // Could fetch full task details, but for now use ID
                    dependencies.push(TaskDependency {
                        prerequisite_description: format!("Similar to task: {}", dep_id),
                        prerequisite_keywords: Vec::new(),
                        confidence: 0.6, // Lower confidence for historical matches
                        typical_gap_hours: 12.0,
                    });
                }
            }
        }

        // Deduplicate and sort by confidence
        dependencies.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        dependencies.dedup_by(|a, b| a.prerequisite_keywords == b.prerequisite_keywords);

        Ok(dependencies)
    }

    /// Build dependency graph from a set of tasks
    pub async fn build_dependency_graph(&self, tasks: &[TodoTask]) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        for task in tasks {
            graph.add_node(task.id.clone());
        }

        // Find dependencies between tasks
        for i in 0..tasks.len() {
            let task = &tasks[i];
            let deps = self.find_dependencies(task).await?;

            for dep in deps {
                // Find matching tasks in our set
                for j in 0..tasks.len() {
                    if i == j {
                        continue;
                    }

                    let potential_prereq = &tasks[j];
                    let prereq_features = TaskFeatures::extract(&potential_prereq.description);

                    // Check if this task matches the dependency
                    let matches = dep.prerequisite_keywords.iter()
                        .any(|keyword| prereq_features.keywords.contains(keyword));

                    if matches {
                        // Add edge from prerequisite to dependent
                        graph.add_edge(potential_prereq.id.clone(), task.id.clone());
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Train the dependency learner from historical data
    pub async fn train(&mut self) -> Result<()> {
        let executions = self.history.get_successful_executions().await?;

        if executions.len() < 10 {
            return Ok(()); // Not enough data
        }

        // Analyze temporal patterns in task execution
        let mut temporal_patterns: HashMap<(String, String), Vec<f64>> = HashMap::new();

        for i in 0..executions.len() {
            for j in i + 1..executions.len() {
                let earlier = &executions[i];
                let later = &executions[j];

                // Calculate time gap in hours
                if let (Some(earlier_completed), Some(later_started)) = (
                    earlier.completed_at,
                    Some(later.started_at),
                ) {
                    let gap_hours = (later_started - earlier_completed).num_hours() as f64;

                    if gap_hours >= 0.0 && gap_hours < 168.0 {
                        // Within a week
                        let earlier_features = TaskFeatures::extract(&earlier.description);
                        let later_features = TaskFeatures::extract(&later.description);

                        // Look for keyword patterns
                        for prereq_kw in &earlier_features.keywords {
                            for dep_kw in &later_features.keywords {
                                temporal_patterns
                                    .entry((prereq_kw.clone(), dep_kw.clone()))
                                    .or_insert_with(Vec::new)
                                    .push(gap_hours);
                            }
                        }
                    }
                }
            }
        }

        // Find frequent patterns
        let mut learned_rules = Vec::new();

        for ((prereq_kw, dep_kw), gaps) in temporal_patterns {
            if gaps.len() >= 3 {
                // At least 3 occurrences
                let avg_gap: f64 = gaps.iter().sum::<f64>() / gaps.len() as f64;
                let confidence = (gaps.len() as f64 / executions.len() as f64).min(0.9);

                learned_rules.push(DependencyRule {
                    name: format!("{} before {}", prereq_kw, dep_kw),
                    prerequisite_keywords: vec![prereq_kw],
                    dependent_keywords: vec![dep_kw],
                    confidence,
                    occurrence_count: gaps.len(),
                });
            }
        }

        // Merge with existing rules (keep higher confidence)
        for learned_rule in learned_rules {
            if let Some(existing) = self.rules.iter_mut().find(|r| r.name == learned_rule.name) {
                if learned_rule.confidence > existing.confidence {
                    *existing = learned_rule;
                }
            } else {
                self.rules.push(learned_rule);
            }
        }

        // Sort rules by confidence
        self.rules.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        tracing::info!("Learned {} dependency rules", self.rules.len());

        Ok(())
    }

    /// Get count of learned dependency rules
    pub async fn get_rule_count(&self) -> Result<usize> {
        Ok(self.rules.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        graph.add_edge("task1".to_string(), "task2".to_string());
        graph.add_edge("task2".to_string(), "task3".to_string());
        graph.add_edge("task1".to_string(), "task4".to_string());

        let sorted = graph.topological_sort().unwrap();

        // task1 should come before task2, task2 before task3
        let pos1 = sorted.iter().position(|t| t == "task1").unwrap();
        let pos2 = sorted.iter().position(|t| t == "task2").unwrap();
        let pos3 = sorted.iter().position(|t| t == "task3").unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_default_rules() {
        let rules = DependencyLearner::default_rules();

        assert!(!rules.is_empty());

        // Check "Design before Implementation" rule exists
        let design_rule = rules.iter()
            .find(|r| r.name == "Design before Implementation");
        assert!(design_rule.is_some());

        let rule = design_rule.unwrap();
        assert!(rule.confidence > 0.8);
    }
}
