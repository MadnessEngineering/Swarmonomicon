/// Consensus Protocols for Multi-Agent Decision Making
///
/// Enables agents to reach collective decisions through voting and agreement

use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Voting strategy for consensus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VotingStrategy {
    Majority,    // >50% agreement
    Plurality,   // Most votes wins
    Unanimous,   // 100% agreement
    Weighted,    // Votes weighted by agent expertise
}

/// A consensus decision
#[derive(Debug, Clone)]
pub struct ConsensusDecision<A> {
    pub action: A,
    pub agreement_score: f64, // 0.0 to 1.0
    pub votes: HashMap<String, A>, // agent_id -> their vote
    pub strategy_used: VotingStrategy,
}

/// Consensus protocol for swarm decisions
pub struct ConsensusProtocol {
    threshold: f64, // Agreement threshold (0.0-1.0)
    strategy: VotingStrategy,
    agent_weights: HashMap<String, f64>, // For weighted voting
}

impl ConsensusProtocol {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            strategy: VotingStrategy::Majority,
            agent_weights: HashMap::new(),
        }
    }

    /// Set voting strategy
    pub fn set_strategy(&mut self, strategy: VotingStrategy) {
        self.strategy = strategy;
    }

    /// Set agent weight for weighted voting
    pub fn set_agent_weight(&mut self, agent_id: String, weight: f64) {
        self.agent_weights.insert(agent_id, weight);
    }

    /// Reach consensus on an action
    pub async fn reach_consensus<S, A>(
        &self,
        agents: &[String],
        _state: &S,
        valid_actions: &[A],
    ) -> Result<Option<ConsensusDecision<A>>>
    where
        S: Clone + std::fmt::Debug,
        A: Clone + std::fmt::Debug + PartialEq,
    {
        // Simulate agents voting (in real implementation, poll actual agents)
        let votes = self.simulate_votes(agents, valid_actions);

        // Count votes
        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        for action in votes.values() {
            let key = format!("{:?}", action);
            *vote_counts.entry(key).or_insert(0) += 1;
        }

        // Find winning action
        let total_votes = agents.len();
        let (winner_key, winner_count) = vote_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(k, c)| (k.clone(), *c))
            .ok_or_else(|| anyhow::anyhow!("No votes cast"))?;

        let agreement_score = winner_count as f64 / total_votes as f64;

        // Check if consensus reached
        let consensus_reached = match self.strategy {
            VotingStrategy::Majority => agreement_score > 0.5,
            VotingStrategy::Plurality => true, // Winner always wins
            VotingStrategy::Unanimous => agreement_score == 1.0,
            VotingStrategy::Weighted => agreement_score >= self.threshold,
        };

        if consensus_reached {
            // Find the actual action object
            let winning_action = votes.values()
                .find(|a| format!("{:?}", a) == winner_key)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Winner action not found"))?;

            Ok(Some(ConsensusDecision {
                action: winning_action,
                agreement_score,
                votes,
                strategy_used: self.strategy.clone(),
            }))
        } else {
            Ok(None) // No consensus
        }
    }

    /// Simulate agent votes (placeholder - real implementation would poll agents)
    fn simulate_votes<A>(&self, agents: &[String], valid_actions: &[A]) -> HashMap<String, A>
    where
        A: Clone,
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        agents.iter()
            .map(|agent_id| {
                let action_idx = rng.gen_range(0..valid_actions.len());
                (agent_id.clone(), valid_actions[action_idx].clone())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestAction {
        A,
        B,
        C,
    }

    #[tokio::test]
    async fn test_consensus_protocol() -> Result<()> {
        let protocol = ConsensusProtocol::new(0.6);

        let agents = vec!["agent1".to_string(), "agent2".to_string(), "agent3".to_string()];
        let actions = vec![TestAction::A, TestAction::B, TestAction::C];
        let state = "test_state";

        let decision = protocol.reach_consensus(&agents, &state, &actions).await?;

        if let Some(consensus) = decision {
            assert!(consensus.agreement_score > 0.0);
            assert_eq!(consensus.votes.len(), 3);
        }

        Ok(())
    }
}
