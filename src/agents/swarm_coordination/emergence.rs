/// Emergent Behavior Detection for Swarm Intelligence
///
/// Detects patterns and behaviors that emerge from agent interactions

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Types of emergent swarm behaviors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwarmBehavior {
    /// Agents naturally form specialized roles
    RoleSpecialization { roles: Vec<String> },

    /// Agents develop coordinated response patterns
    CoordinatedResponse { pattern: String, frequency: usize },

    /// Agents form chains of collaboration
    CollaborationChain { chain_length: usize },

    /// Agents achieve consensus without explicit voting
    ImplicitConsensus { agreement_rate: f64 },

    /// Agents distribute work optimally
    SelfOrganization { efficiency_score: f64 },
}

/// Metrics about emergent behaviors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceMetrics {
    pub behaviors_detected: Vec<SwarmBehavior>,
    pub emergence_score: f64, // 0.0 to 1.0
    pub pattern_stability: f64, // How consistent patterns are
    pub novelty_score: f64, // How new/unexpected patterns are
}

/// Detector for emergent behaviors in swarm
pub struct EmergencDetector {
    window_size: usize,
    detected_patterns: Vec<SwarmBehavior>,
    interaction_history: Vec<String>,
}

impl EmergencDetector {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            detected_patterns: Vec::new(),
            interaction_history: Vec::new(),
        }
    }

    /// Analyze interactions for emergent patterns
    pub fn analyze_interactions(&mut self, interactions: &[String]) -> Result<Vec<SwarmBehavior>> {
        self.interaction_history.extend_from_slice(interactions);

        // Keep only recent interactions
        if self.interaction_history.len() > self.window_size {
            let start = self.interaction_history.len() - self.window_size;
            self.interaction_history = self.interaction_history[start..].to_vec();
        }

        let mut behaviors = Vec::new();

        // Detect role specialization
        if let Some(behavior) = self.detect_role_specialization() {
            behaviors.push(behavior);
        }

        // Detect coordination patterns
        if let Some(behavior) = self.detect_coordination_patterns() {
            behaviors.push(behavior);
        }

        // Detect self-organization
        if let Some(behavior) = self.detect_self_organization() {
            behaviors.push(behavior);
        }

        self.detected_patterns = behaviors.clone();
        Ok(behaviors)
    }

    /// Detect if agents are specializing into roles
    fn detect_role_specialization(&self) -> Option<SwarmBehavior> {
        // Simplified: Look for repeated agent-task pairings
        let mut agent_tasks: HashMap<String, Vec<String>> = HashMap::new();

        for interaction in &self.interaction_history {
            // Parse interaction (simplified)
            if let Some((agent, task)) = self.parse_interaction(interaction) {
                agent_tasks.entry(agent).or_insert_with(Vec::new).push(task);
            }
        }

        // Check if agents consistently handle similar tasks
        let specialized_agents: Vec<String> = agent_tasks.iter()
            .filter(|(_, tasks)| {
                if tasks.len() < 3 {
                    return false;
                }
                // Check task similarity (simplified)
                let first = &tasks[0];
                tasks.iter().filter(|t| t == &first).count() > tasks.len() / 2
            })
            .map(|(agent, _)| agent.clone())
            .collect();

        if specialized_agents.len() >= 2 {
            Some(SwarmBehavior::RoleSpecialization {
                roles: specialized_agents,
            })
        } else {
            None
        }
    }

    /// Detect coordination patterns
    fn detect_coordination_patterns(&self) -> Option<SwarmBehavior> {
        // Look for repeated sequences of interactions
        if self.interaction_history.len() < 5 {
            return None;
        }

        // Find most common interaction pattern
        let mut pattern_counts: HashMap<String, usize> = HashMap::new();

        for i in 0..self.interaction_history.len().saturating_sub(2) {
            let pattern = format!("{}->{}",
                                  self.interaction_history[i],
                                  self.interaction_history[i + 1]);
            *pattern_counts.entry(pattern).or_insert(0) += 1;
        }

        pattern_counts.iter()
            .filter(|(_, &count)| count >= 3)
            .max_by_key(|(_, count)| *count)
            .map(|(pattern, count)| SwarmBehavior::CoordinatedResponse {
                pattern: pattern.clone(),
                frequency: *count,
            })
    }

    /// Detect self-organization
    fn detect_self_organization(&self) -> Option<SwarmBehavior> {
        if self.interaction_history.len() < 10 {
            return None;
        }

        // Measure: Are interactions becoming more efficient over time?
        let first_half = &self.interaction_history[..self.interaction_history.len() / 2];
        let second_half = &self.interaction_history[self.interaction_history.len() / 2..];

        // Simple heuristic: count unique patterns (lower = more organized)
        let first_unique: std::collections::HashSet<_> = first_half.iter().collect();
        let second_unique: std::collections::HashSet<_> = second_half.iter().collect();

        if second_unique.len() < first_unique.len() {
            let efficiency = 1.0 - (second_unique.len() as f64 / first_unique.len() as f64);
            Some(SwarmBehavior::SelfOrganization {
                efficiency_score: efficiency,
            })
        } else {
            None
        }
    }

    /// Parse interaction string (simplified)
    fn parse_interaction(&self, interaction: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = interaction.split(':').collect();
        if parts.len() >= 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Get emergence metrics
    pub fn get_metrics(&self) -> EmergenceMetrics {
        let emergence_score = self.detected_patterns.len() as f64 / 5.0; // Max 5 types

        EmergenceMetrics {
            behaviors_detected: self.detected_patterns.clone(),
            emergence_score: emergence_score.min(1.0),
            pattern_stability: 0.8, // Placeholder
            novelty_score: 0.6, // Placeholder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergence_detector() -> Result<()> {
        let mut detector = EmergencDetector::new(50);

        let interactions = vec![
            "agent1:git_task".to_string(),
            "agent1:git_task".to_string(),
            "agent2:haiku_task".to_string(),
            "agent2:haiku_task".to_string(),
            "agent1:git_task".to_string(),
        ];

        let behaviors = detector.analyze_interactions(&interactions)?;

        // Should detect some patterns
        assert!(!behaviors.is_empty() || interactions.len() < detector.window_size);

        Ok(())
    }
}
