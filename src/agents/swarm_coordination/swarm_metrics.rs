/// Swarm Performance Metrics and Tracking
///
/// Measures swarm intelligence, collaboration, and collective performance

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Individual agent's contribution to swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContribution {
    pub agent_id: String,
    pub tasks_completed: usize,
    pub success_rate: f64,
    pub collaboration_events: usize, // Times collaborated with others
    pub leadership_score: f64, // How often agent leads decisions
    pub consensus_participation: usize,
}

/// Collaboration score between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationScore {
    pub agent_pair: (String, String),
    pub interaction_count: usize,
    pub success_rate: f64,
    pub synergy_score: f64, // Performance boost from collaboration
}

/// Overall swarm performance tracking
#[derive(Debug, Clone)]
pub struct SwarmPerformance {
    pub total_decisions: usize,
    pub consensus_count: usize,
    pub delegation_count: usize,
    pub emergence_count: usize,
    agent_contributions: HashMap<String, AgentContribution>,
    collaboration_scores: Vec<CollaborationScore>,
}

impl SwarmPerformance {
    pub fn new() -> Self {
        Self {
            total_decisions: 0,
            consensus_count: 0,
            delegation_count: 0,
            emergence_count: 0,
            agent_contributions: HashMap::new(),
            collaboration_scores: Vec::new(),
        }
    }

    /// Record a swarm decision
    pub fn record_swarm_decision(&mut self, consensus_reached: bool) {
        self.total_decisions += 1;
        if consensus_reached {
            self.consensus_count += 1;
        }
    }

    /// Record a task delegation
    pub fn record_delegation(&mut self) {
        self.delegation_count += 1;
    }

    /// Record emergent behaviors detected
    pub fn record_emergent_behaviors(&mut self, count: usize) {
        self.emergence_count += count;
    }

    /// Update agent contribution
    pub fn update_agent_contribution(
        &mut self,
        agent_id: &str,
        task_success: bool,
        collaborated: bool,
        led_decision: bool,
    ) {
        let contrib = self.agent_contributions
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentContribution {
                agent_id: agent_id.to_string(),
                tasks_completed: 0,
                success_rate: 0.0,
                collaboration_events: 0,
                leadership_score: 0.0,
                consensus_participation: 0,
            });

        // Update task stats
        let old_total = contrib.tasks_completed as f64;
        let old_successes = old_total * contrib.success_rate;
        contrib.tasks_completed += 1;
        let new_successes = if task_success { old_successes + 1.0 } else { old_successes };
        contrib.success_rate = new_successes / contrib.tasks_completed as f64;

        if collaborated {
            contrib.collaboration_events += 1;
        }

        if led_decision {
            let old_leadership = contrib.leadership_score * old_total;
            contrib.leadership_score = (old_leadership + 1.0) / (old_total + 1.0);
        }
    }

    /// Calculate average swarm performance
    pub fn avg_performance(&self) -> f64 {
        if self.agent_contributions.is_empty() {
            return 0.0;
        }

        let total_success: f64 = self.agent_contributions.values()
            .map(|c| c.success_rate)
            .sum();

        total_success / self.agent_contributions.len() as f64
    }

    /// Get count of active agents
    pub fn active_agent_count(&self) -> usize {
        self.agent_contributions.len()
    }

    /// Calculate collaboration score
    pub fn collaboration_score(&self) -> f64 {
        if self.agent_contributions.is_empty() {
            return 0.0;
        }

        let total_collab: usize = self.agent_contributions.values()
            .map(|c| c.collaboration_events)
            .sum();

        let total_tasks: usize = self.agent_contributions.values()
            .map(|c| c.tasks_completed)
            .sum();

        if total_tasks == 0 {
            0.0
        } else {
            total_collab as f64 / total_tasks as f64
        }
    }

    /// Get top contributors
    pub fn top_contributors(&self, n: usize) -> Vec<AgentContribution> {
        let mut contribs: Vec<_> = self.agent_contributions.values().cloned().collect();
        contribs.sort_by(|a, b| {
            b.tasks_completed.cmp(&a.tasks_completed)
                .then(b.success_rate.partial_cmp(&a.success_rate).unwrap())
        });
        contribs.into_iter().take(n).collect()
    }

    /// Get consensus rate
    pub fn consensus_rate(&self) -> f64 {
        if self.total_decisions == 0 {
            0.0
        } else {
            self.consensus_count as f64 / self.total_decisions as f64
        }
    }
}

impl Default for SwarmPerformance {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_performance() {
        let mut perf = SwarmPerformance::new();

        perf.record_swarm_decision(true);
        perf.record_swarm_decision(false);
        perf.record_delegation();

        assert_eq!(perf.total_decisions, 2);
        assert_eq!(perf.consensus_count, 1);
        assert_eq!(perf.delegation_count, 1);

        assert_eq!(perf.consensus_rate(), 0.5);
    }

    #[test]
    fn test_agent_contribution() {
        let mut perf = SwarmPerformance::new();

        perf.update_agent_contribution("agent1", true, true, false);
        perf.update_agent_contribution("agent1", true, false, true);
        perf.update_agent_contribution("agent2", false, true, false);

        assert_eq!(perf.active_agent_count(), 2);

        let contrib1 = &perf.agent_contributions["agent1"];
        assert_eq!(contrib1.tasks_completed, 2);
        assert_eq!(contrib1.success_rate, 1.0);
        assert_eq!(contrib1.collaboration_events, 1);
    }
}
