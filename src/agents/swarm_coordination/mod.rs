/// Multi-Agent RL Coordination System
///
/// Enables collaborative learning and emergent swarm intelligence:
/// - Shared Q-learning across multiple agents
/// - Consensus protocols for collective decision-making
/// - Delegation strategies for task distribution
/// - Emergent behavior detection and measurement
/// - Swarm performance metrics
///
/// This system builds on the existing RL infrastructure to enable
/// true multi-agent reinforcement learning and swarm intelligence.

pub mod shared_learning;
pub mod consensus;
pub mod delegation;
pub mod emergence;
pub mod swarm_metrics;

pub use shared_learning::{SharedQLearning, SharedState, SharedAction};
pub use consensus::{ConsensusProtocol, VotingStrategy, ConsensusDecision};
pub use delegation::{DelegationStrategy, TaskAssignment, SpecializationProfile};
pub use emergence::{EmergencDetector, SwarmBehavior, EmergenceMetrics};
pub use swarm_metrics::{SwarmPerformance, AgentContribution, CollaborationScore};

use anyhow::Result;
use mongodb::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Configuration for swarm coordination
#[derive(Debug, Clone)]
pub struct SwarmCoordinationConfig {
    pub enabled: bool,
    pub mongo_client: Option<Client>,

    // Feature flags
    pub enable_shared_learning: bool,
    pub enable_consensus: bool,
    pub enable_delegation: bool,
    pub enable_emergence_detection: bool,

    // Swarm parameters
    pub min_agents_for_swarm: usize, // Minimum agents to form a swarm
    pub consensus_threshold: f64, // Agreement threshold (0.0-1.0)
    pub delegation_strategy: String, // "expertise", "availability", "round_robin"
    pub emergence_window: usize, // Interactions to analyze for emergence
}

impl SwarmCoordinationConfig {
    pub fn new(mongo_client: Client) -> Self {
        Self {
            enabled: true,
            mongo_client: Some(mongo_client),
            enable_shared_learning: true,
            enable_consensus: true,
            enable_delegation: true,
            enable_emergence_detection: true,
            min_agents_for_swarm: 2,
            consensus_threshold: 0.6, // 60% agreement
            delegation_strategy: "expertise".to_string(),
            emergence_window: 100,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            mongo_client: None,
            enable_shared_learning: false,
            enable_consensus: false,
            enable_delegation: false,
            enable_emergence_detection: false,
            min_agents_for_swarm: 2,
            consensus_threshold: 0.6,
            delegation_strategy: "round_robin".to_string(),
            emergence_window: 100,
        }
    }
}

impl Default for SwarmCoordinationConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Statistics about swarm coordination
#[derive(Debug, Clone)]
pub struct SwarmCoordinationStats {
    pub total_swarm_decisions: usize,
    pub consensus_reached: usize,
    pub delegations_made: usize,
    pub emergent_behaviors_detected: usize,
    pub avg_swarm_performance: f64,
    pub active_agents: usize,
    pub collaboration_score: f64,
}

/// Main swarm coordination service
pub struct SwarmCoordinator {
    config: SwarmCoordinationConfig,
    shared_learning: Option<Arc<RwLock<SharedQLearning>>>,
    consensus_protocol: Option<Arc<RwLock<ConsensusProtocol>>>,
    delegation_strategy: Option<Arc<RwLock<DelegationStrategy>>>,
    emergence_detector: Option<Arc<RwLock<EmergencDetector>>>,
    metrics: Arc<RwLock<SwarmPerformance>>,
}

impl SwarmCoordinator {
    pub async fn new(config: SwarmCoordinationConfig) -> Result<Self> {
        if !config.enabled {
            return Ok(Self {
                config,
                shared_learning: None,
                consensus_protocol: None,
                delegation_strategy: None,
                emergence_detector: None,
                metrics: Arc::new(RwLock::new(SwarmPerformance::new())),
            });
        }

        let mongo_client = config.mongo_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("MongoDB client required when swarm coordination is enabled"))?;

        // Initialize shared learning
        let shared_learning = if config.enable_shared_learning {
            Some(Arc::new(RwLock::new(
                SharedQLearning::new(mongo_client.clone()).await?
            )))
        } else {
            None
        };

        // Initialize consensus protocol
        let consensus_protocol = if config.enable_consensus {
            Some(Arc::new(RwLock::new(
                ConsensusProtocol::new(config.consensus_threshold)
            )))
        } else {
            None
        };

        // Initialize delegation strategy
        let delegation_strategy = if config.enable_delegation {
            Some(Arc::new(RwLock::new(
                DelegationStrategy::new(mongo_client.clone()).await?
            )))
        } else {
            None
        };

        // Initialize emergence detector
        let emergence_detector = if config.enable_emergence_detection {
            Some(Arc::new(RwLock::new(
                EmergencDetector::new(config.emergence_window)
            )))
        } else {
            None
        };

        Ok(Self {
            config,
            shared_learning,
            consensus_protocol,
            delegation_strategy,
            emergence_detector,
            metrics: Arc::new(RwLock::new(SwarmPerformance::new())),
        })
    }

    /// Make a decision as a swarm using consensus
    pub async fn swarm_decide<S, A>(&self, agents: &[String], state: &S, valid_actions: &[A]) -> Result<Option<A>>
    where
        S: Clone + std::fmt::Debug,
        A: Clone + std::fmt::Debug + PartialEq,
    {
        if let Some(consensus) = &self.consensus_protocol {
            let decision = consensus.write().await.reach_consensus(agents, state, valid_actions).await?;

            // Update metrics
            self.metrics.write().await.record_swarm_decision(decision.is_some());

            Ok(decision.map(|d| d.action))
        } else {
            Ok(None)
        }
    }

    /// Delegate a task to the best agent
    pub async fn delegate_task(&self, task_description: &str, available_agents: &[String]) -> Result<Option<String>> {
        if let Some(delegator) = &self.delegation_strategy {
            let assignment = delegator.write().await
                .assign_task(task_description, available_agents)
                .await?;

            // Update metrics
            self.metrics.write().await.record_delegation();

            Ok(Some(assignment.assigned_agent))
        } else {
            Ok(None)
        }
    }

    /// Record experience in shared Q-table
    pub async fn record_shared_experience<S, A>(
        &self,
        agent_id: &str,
        state: &S,
        action: &A,
        reward: f64,
        next_state: &S,
    ) -> Result<()>
    where
        S: shared_learning::SharedState,
        A: shared_learning::SharedAction,
    {
        if let Some(shared) = &self.shared_learning {
            shared.write().await.update(agent_id, state, action, reward, next_state).await?;
        }
        Ok(())
    }

    /// Detect emergent behaviors in swarm
    pub async fn detect_emergence(&self, interaction_history: &[String]) -> Result<Vec<SwarmBehavior>> {
        if let Some(detector) = &self.emergence_detector {
            let behaviors = detector.write().await.analyze_interactions(interaction_history)?;

            // Update metrics
            let count = behaviors.len();
            self.metrics.write().await.record_emergent_behaviors(count);

            Ok(behaviors)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get swarm coordination statistics
    pub async fn get_stats(&self) -> Result<SwarmCoordinationStats> {
        let metrics = self.metrics.read().await;

        Ok(SwarmCoordinationStats {
            total_swarm_decisions: metrics.total_decisions,
            consensus_reached: metrics.consensus_count,
            delegations_made: metrics.delegation_count,
            emergent_behaviors_detected: metrics.emergence_count,
            avg_swarm_performance: metrics.avg_performance(),
            active_agents: metrics.active_agent_count(),
            collaboration_score: metrics.collaboration_score(),
        })
    }

    /// Train shared Q-learning from collective experience
    pub async fn train_swarm(&self) -> Result<()> {
        if let Some(shared) = &self.shared_learning {
            shared.write().await.train_from_collective_experience().await?;
        }
        Ok(())
    }

    /// Get agent specialization profiles
    pub async fn get_specializations(&self) -> Result<HashMap<String, SpecializationProfile>> {
        if let Some(delegator) = &self.delegation_strategy {
            Ok(delegator.read().await.get_all_profiles())
        } else {
            Ok(HashMap::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swarm_coordinator_creation() -> Result<()> {
        // Test disabled config
        let config = SwarmCoordinationConfig::disabled();
        let coordinator = SwarmCoordinator::new(config).await?;

        assert!(coordinator.shared_learning.is_none());
        assert!(coordinator.consensus_protocol.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_swarm_stats() -> Result<()> {
        let config = SwarmCoordinationConfig::disabled();
        let coordinator = SwarmCoordinator::new(config).await?;

        let stats = coordinator.get_stats().await?;
        assert_eq!(stats.total_swarm_decisions, 0);
        assert_eq!(stats.emergent_behaviors_detected, 0);

        Ok(())
    }
}
