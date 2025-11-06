use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use super::interaction::InteractionTracker;

#[cfg(feature = "rl")]
use crate::agents::rl::{State, Action, QLearningAgent};

/// State representation for agent routing
#[cfg_attr(feature = "rl", derive(Hash, Eq, PartialEq))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingState {
    pub current_agent: String,
    pub user_intent: String, // Simplified intent category
    pub conversation_depth: i32, // Number of turns
    pub last_success: bool,
}

#[cfg(feature = "rl")]
impl State for RoutingState {
    fn to_features(&self) -> Vec<f64> {
        vec![
            // One-hot encode current agent (simplified - in reality use hash or embedding)
            self.agent_to_feature(&self.current_agent),
            // Intent category
            self.intent_to_feature(&self.user_intent),
            // Conversation depth normalized
            (self.conversation_depth as f64 / 10.0).min(1.0),
            // Last success as binary
            if self.last_success { 1.0 } else { 0.0 },
        ]
    }
}

impl RoutingState {
    fn agent_to_feature(&self, agent: &str) -> f64 {
        // Simple hash-based encoding (in production, use embeddings)
        let hash: u32 = agent.bytes().map(|b| b as u32).sum();
        (hash % 100) as f64 / 100.0
    }

    fn intent_to_feature(&self, intent: &str) -> f64 {
        match intent {
            "greeting" => 0.0,
            "git_help" => 0.2,
            "project_init" => 0.4,
            "creative" => 0.6,
            "general" => 0.8,
            _ => 1.0,
        }
    }

    /// Classify user intent from message content
    pub fn classify_intent(message: &str) -> String {
        let msg_lower = message.to_lowercase();

        if msg_lower.contains("hello") || msg_lower.contains("hi") {
            "greeting".to_string()
        } else if msg_lower.contains("git") || msg_lower.contains("commit") || msg_lower.contains("branch") {
            "git_help".to_string()
        } else if msg_lower.contains("project") || msg_lower.contains("init") || msg_lower.contains("create") {
            "project_init".to_string()
        } else if msg_lower.contains("haiku") || msg_lower.contains("poetry") || msg_lower.contains("creative") {
            "creative".to_string()
        } else {
            "general".to_string()
        }
    }
}

/// Routing actions (which agent to transfer to)
#[cfg_attr(feature = "rl", derive(Hash, Eq, PartialEq))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingAction {
    StayWithCurrent,
    TransferToGreeter,
    TransferToGit,
    TransferToProject,
    TransferToHaiku,
}

#[cfg(feature = "rl")]
impl Action for RoutingAction {
    fn to_index(&self) -> usize {
        match self {
            RoutingAction::StayWithCurrent => 0,
            RoutingAction::TransferToGreeter => 1,
            RoutingAction::TransferToGit => 2,
            RoutingAction::TransferToProject => 3,
            RoutingAction::TransferToHaiku => 4,
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(RoutingAction::StayWithCurrent),
            1 => Some(RoutingAction::TransferToGreeter),
            2 => Some(RoutingAction::TransferToGit),
            3 => Some(RoutingAction::TransferToProject),
            4 => Some(RoutingAction::TransferToHaiku),
            _ => None,
        }
    }
}

impl RoutingAction {
    pub fn to_agent_name(&self, current_agent: &str) -> String {
        match self {
            RoutingAction::StayWithCurrent => current_agent.to_string(),
            RoutingAction::TransferToGreeter => "greeter".to_string(),
            RoutingAction::TransferToGit => "git".to_string(),
            RoutingAction::TransferToProject => "project".to_string(),
            RoutingAction::TransferToHaiku => "haiku".to_string(),
        }
    }
}

/// RL-based agent routing policy
pub struct AgentRoutingPolicy {
    #[cfg(feature = "rl")]
    agent: QLearningAgent<RoutingState, RoutingAction>,
    tracker: InteractionTracker,
    learning_enabled: bool,
}

impl AgentRoutingPolicy {
    pub fn new(tracker: InteractionTracker) -> Self {
        #[cfg(feature = "rl")]
        let agent = QLearningAgent::new(
            0.1,  // learning_rate
            0.95, // discount_factor
            0.2,  // epsilon for exploration
        );

        Self {
            #[cfg(feature = "rl")]
            agent,
            tracker,
            learning_enabled: true,
        }
    }

    /// Load a pre-trained model
    #[cfg(feature = "rl")]
    pub async fn load_model(tracker: InteractionTracker, model_path: &str) -> Result<Self> {
        let mut agent = QLearningAgent::new(0.1, 0.95, 0.1);
        agent.load_model(model_path).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Self {
            agent,
            tracker,
            learning_enabled: true,
        })
    }

    /// Decide best routing action based on current state
    pub fn decide_routing(
        &mut self,
        current_agent: &str,
        user_message: &str,
        conversation_depth: i32,
        last_success: bool,
    ) -> String {
        #[cfg(feature = "rl")]
        {
            let state = RoutingState {
                current_agent: current_agent.to_string(),
                user_intent: RoutingState::classify_intent(user_message),
                conversation_depth,
                last_success,
            };

            let valid_actions = vec![
                RoutingAction::StayWithCurrent,
                RoutingAction::TransferToGreeter,
                RoutingAction::TransferToGit,
                RoutingAction::TransferToProject,
                RoutingAction::TransferToHaiku,
            ];

            let action = self.agent.choose_action(&state, &valid_actions);
            return action.to_agent_name(current_agent);
        }

        #[cfg(not(feature = "rl"))]
        {
            // Fallback: simple rule-based routing
            let intent = RoutingState::classify_intent(user_message);
            match intent.as_str() {
                "git_help" => "git".to_string(),
                "project_init" => "project".to_string(),
                "creative" => "haiku".to_string(),
                _ => current_agent.to_string(),
            }
        }
    }

    /// Update the policy based on routing outcome
    #[cfg(feature = "rl")]
    pub async fn update_from_outcome(
        &mut self,
        state: RoutingState,
        action: RoutingAction,
        success: bool,
        user_satisfaction: Option<f64>,
    ) {
        if !self.learning_enabled {
            return;
        }

        // Calculate reward based on success and satisfaction
        let reward = if success {
            user_satisfaction.unwrap_or(0.7) * 10.0 // Scale to make learning faster
        } else {
            -5.0 // Penalty for unsuccessful routing
        };

        // Create next state (simplified - in reality track actual next state)
        let next_state = RoutingState {
            current_agent: action.to_agent_name(&state.current_agent),
            user_intent: state.user_intent.clone(),
            conversation_depth: state.conversation_depth + 1,
            last_success: success,
        };

        // Update Q-values
        self.agent.update(&state, &action, reward, &next_state);
    }

    /// Train the policy from historical interactions
    pub async fn train_from_history(&mut self, user_id: &str, episodes: usize) -> Result<()> {
        let history = self.tracker.get_user_history(user_id, 500).await?;

        if history.is_empty() {
            return Ok(());
        }

        // Train over multiple episodes
        for _ in 0..episodes {
            for i in 0..history.len().saturating_sub(1) {
                let current = &history[i];
                let next = &history[i + 1];

                // Build state
                let state = RoutingState {
                    current_agent: current.agent_name.clone(),
                    user_intent: RoutingState::classify_intent(&current.message_content),
                    conversation_depth: i as i32,
                    last_success: current.success,
                };

                // Determine action taken
                let action = if current.transfer_occurred {
                    if let Some(ref target) = current.target_agent {
                        match target.as_str() {
                            "greeter" => RoutingAction::TransferToGreeter,
                            "git" => RoutingAction::TransferToGit,
                            "project" => RoutingAction::TransferToProject,
                            "haiku" => RoutingAction::TransferToHaiku,
                            _ => RoutingAction::StayWithCurrent,
                        }
                    } else {
                        RoutingAction::StayWithCurrent
                    }
                } else {
                    RoutingAction::StayWithCurrent
                };

                // Calculate reward
                let reward = if next.success {
                    next.user_satisfaction.unwrap_or(0.7) * 10.0
                } else {
                    -5.0
                };

                // Next state
                let next_state = RoutingState {
                    current_agent: next.agent_name.clone(),
                    user_intent: RoutingState::classify_intent(&next.message_content),
                    conversation_depth: (i + 1) as i32,
                    last_success: next.success,
                };

                // Update
                self.agent.update(&state, &action, reward, &next_state);
            }
        }

        Ok(())
    }

    /// Save the trained model
    #[cfg(feature = "rl")]
    pub async fn save_model(&self, model_path: &str) -> Result<()> {
        self.agent.save_model(model_path).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    /// Enable or disable learning
    pub fn set_learning_enabled(&mut self, enabled: bool) {
        self.learning_enabled = enabled;
    }

    /// Get current exploration rate (epsilon)
    #[cfg(feature = "rl")]
    pub fn get_exploration_rate(&self) -> f64 {
        self.agent.metadata.epsilon
    }

    /// Decay exploration rate over time
    #[cfg(feature = "rl")]
    pub fn decay_exploration(&mut self, decay_rate: f64) {
        let config = crate::agents::rl::model::config::TrainingConfig {
            epsilon_decay: decay_rate,
            min_epsilon: 0.01,
            ..self.agent.get_config()
        };
        self.agent.decay_epsilon(&config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::Client;

    #[tokio::test]
    async fn test_routing_policy() -> Result<()> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let tracker = InteractionTracker::new(&client).await?;
        let mut policy = AgentRoutingPolicy::new(tracker);

        // Test routing decision
        let target_agent = policy.decide_routing(
            "greeter",
            "I need help with git",
            1,
            true,
        );

        // Should route toward git-related agent
        assert!(!target_agent.is_empty());

        // Test state classification
        assert_eq!(RoutingState::classify_intent("I need help with git commit"), "git_help");
        assert_eq!(RoutingState::classify_intent("write me a haiku"), "creative");

        Ok(())
    }
}
