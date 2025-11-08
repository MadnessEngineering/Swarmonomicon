/// Task Delegation Strategies for Swarm Coordination
///
/// Assigns tasks to agents based on expertise, availability, and specialization

use std::collections::HashMap;
use mongodb::{Client, Collection};
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Agent specialization profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializationProfile {
    pub agent_id: String,
    pub expertise_areas: Vec<String>, // ["git", "haiku", "project_init"]
    pub success_rate: f64,
    pub tasks_completed: usize,
    pub avg_completion_time: f64,
    pub current_load: usize, // Number of active tasks
}

impl SpecializationProfile {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            expertise_areas: Vec::new(),
            success_rate: 0.0,
            tasks_completed: 0,
            avg_completion_time: 0.0,
            current_load: 0,
        }
    }

    /// Calculate suitability score for a task
    pub fn suitability_for_task(&self, task_keywords: &[String]) -> f64 {
        // Expertise match
        let expertise_score = task_keywords.iter()
            .filter(|keyword| self.expertise_areas.contains(keyword))
            .count() as f64 / task_keywords.len().max(1) as f64;

        // Success rate weight
        let success_weight = self.success_rate;

        // Load penalty (prefer less loaded agents)
        let load_penalty = 1.0 / (1.0 + self.current_load as f64);

        // Combined score
        expertise_score * 0.5 + success_weight * 0.3 + load_penalty * 0.2
    }
}

/// Task assignment
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    pub task_id: String,
    pub assigned_agent: String,
    pub suitability_score: f64,
    pub assignment_reason: String,
}

/// Delegation strategy for task assignment
pub struct DelegationStrategy {
    profiles: HashMap<String, SpecializationProfile>,
    #[allow(dead_code)]
    collection: Collection<mongodb::bson::Document>,
}

impl DelegationStrategy {
    pub async fn new(mongo_client: Client) -> Result<Self> {
        let db = mongo_client.database("swarmonomicon");
        let collection = db.collection("agent_specializations");

        Ok(Self {
            profiles: HashMap::new(),
            collection,
        })
    }

    /// Assign task to best agent
    pub async fn assign_task(
        &self,
        task_description: &str,
        available_agents: &[String],
    ) -> Result<TaskAssignment> {
        // Extract keywords from task
        let keywords = self.extract_keywords(task_description);

        // Find best agent
        let mut best_agent = None;
        let mut best_score = 0.0;

        for agent_id in available_agents {
            if let Some(profile) = self.profiles.get(agent_id) {
                let score = profile.suitability_for_task(&keywords);
                if score > best_score {
                    best_score = score;
                    best_agent = Some(agent_id.clone());
                }
            }
        }

        // Fallback: round-robin if no profiles
        let assigned = best_agent.unwrap_or_else(|| available_agents[0].clone());

        Ok(TaskAssignment {
            task_id: uuid::Uuid::new_v4().to_string(),
            assigned_agent: assigned.clone(),
            suitability_score: best_score,
            assignment_reason: if best_score > 0.0 {
                "Expertise match".to_string()
            } else {
                "Round-robin fallback".to_string()
            },
        })
    }

    /// Update agent specialization from task outcome
    pub fn update_specialization(
        &mut self,
        agent_id: &str,
        task_keywords: Vec<String>,
        success: bool,
        completion_time: f64,
    ) {
        let profile = self.profiles.entry(agent_id.to_string())
            .or_insert_with(|| SpecializationProfile::new(agent_id.to_string()));

        // Update expertise areas
        for keyword in task_keywords {
            if !profile.expertise_areas.contains(&keyword) {
                profile.expertise_areas.push(keyword);
            }
        }

        // Update success rate
        let total = profile.tasks_completed as f64;
        let new_successes = if success { total * profile.success_rate + 1.0 } else { total * profile.success_rate };
        profile.tasks_completed += 1;
        profile.success_rate = new_successes / profile.tasks_completed as f64;

        // Update avg completion time
        profile.avg_completion_time = (profile.avg_completion_time * total + completion_time) / (total + 1.0);
    }

    /// Get all specialization profiles
    pub fn get_all_profiles(&self) -> HashMap<String, SpecializationProfile> {
        self.profiles.clone()
    }

    /// Extract keywords from task description
    fn extract_keywords(&self, description: &str) -> Vec<String> {
        description
            .to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .take(5)
            .map(|s| s.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialization_profile() {
        let mut profile = SpecializationProfile::new("agent1".to_string());
        profile.expertise_areas = vec!["git".to_string(), "code".to_string()];
        profile.success_rate = 0.9;
        profile.current_load = 1;

        let keywords = vec!["git".to_string(), "commit".to_string()];
        let score = profile.suitability_for_task(&keywords);

        assert!(score > 0.0);
        assert!(score <= 1.0);
    }
}
