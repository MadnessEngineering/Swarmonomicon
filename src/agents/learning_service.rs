use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use anyhow::{Result, anyhow};
use mongodb::Client;

use crate::{
    types::{Message, Agent},
    agents::{AgentRegistry, learning::*},
};

/// Configuration for learning-enabled agents
#[derive(Clone)]
pub struct LearningConfig {
    pub enabled: bool,
    pub mongo_client: Option<Client>,
    pub enable_routing: bool,
    pub enable_personality: bool,
    pub enable_preference: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mongo_client: None,
            enable_routing: true,
            enable_personality: true,
            enable_preference: true,
        }
    }
}

impl LearningConfig {
    pub fn new(mongo_client: Client) -> Self {
        Self {
            enabled: true,
            mongo_client: Some(mongo_client),
            enable_routing: true,
            enable_personality: true,
            enable_preference: true,
        }
    }
}

/// Enhanced transfer service with learning capabilities
pub struct LearningTransferService {
    registry: Arc<RwLock<AgentRegistry>>,
    config: LearningConfig,
    tracker: Option<InteractionTracker>,
    #[cfg(feature = "rl")]
    routing_policy: Option<Arc<RwLock<AgentRoutingPolicy>>>,
    personality_adapter: Option<Arc<RwLock<PersonalityAdapter>>>,
    preference_predictor: Option<Arc<PreferencePredictor>>,
}

impl LearningTransferService {
    pub async fn new(registry: Arc<RwLock<AgentRegistry>>, config: LearningConfig) -> Result<Self> {
        let (tracker, routing_policy, personality_adapter, preference_predictor) = if config.enabled {
            let client = config.mongo_client.as_ref()
                .ok_or_else(|| anyhow!("MongoDB client required when learning is enabled"))?;

            let tracker = InteractionTracker::new(client).await?;

            #[cfg(feature = "rl")]
            let routing_policy = if config.enable_routing {
                Some(Arc::new(RwLock::new(AgentRoutingPolicy::new(tracker.clone()))))
            } else {
                None
            };

            #[cfg(not(feature = "rl"))]
            let routing_policy = None;

            let personality_adapter = if config.enable_personality {
                Some(Arc::new(RwLock::new(PersonalityAdapter::new(tracker.clone()))))
            } else {
                None
            };

            let preference_predictor = if config.enable_preference {
                Some(Arc::new(PreferencePredictor::new(tracker.clone())))
            } else {
                None
            };

            (Some(tracker), routing_policy, personality_adapter, preference_predictor)
        } else {
            (None, None, None, None)
        };

        Ok(Self {
            registry,
            config,
            tracker,
            #[cfg(feature = "rl")]
            routing_policy,
            personality_adapter,
            preference_predictor,
        })
    }

    /// Process a message with learning enhancements
    pub async fn process_message(&self, message: Message, user_id: Option<String>) -> Result<Message> {
        let start_time = Utc::now();
        let current_agent = self.get_current_agent_name().await?;

        // Get personality-adjusted prompt if available
        let adjusted_message = if let (Some(adapter), Some(ref uid)) = (&self.personality_adapter, &user_id) {
            let mut adapter_lock = adapter.write().await;
            // Adapt personality for this user
            adapter_lock.adapt_to_user(&current_agent, uid).await?;
            message
        } else {
            message
        };

        // Process the message
        let agent = self.get_agent(&current_agent).await?;
        let response = agent.process_message(adjusted_message.clone()).await?;

        // Record interaction if learning is enabled
        if let (Some(tracker), Some(user_id)) = (&self.tracker, user_id) {
            let duration_ms = (Utc::now() - start_time).num_milliseconds();

            let mut interaction = UserInteraction::new(
                user_id.clone(),
                current_agent.clone(),
                adjusted_message.content.clone(),
                response.content.clone(),
            ).with_duration(duration_ms);

            // Check if a transfer occurred
            if let Some(ref metadata) = response.metadata {
                if let Some(ref target) = metadata.transfer_target {
                    interaction = interaction.with_transfer(target.clone());
                }
            }

            tracker.record(interaction).await?;
        }

        Ok(response)
    }

    /// Intelligent transfer using RL routing policy
    pub async fn smart_transfer(&self, from: &str, message: Message, user_id: Option<String>) -> Result<Message> {
        // Determine best target agent
        let target_agent = if self.config.enable_routing {
            #[cfg(feature = "rl")]
            {
                if let Some(policy) = &self.routing_policy {
                    let mut policy_lock = policy.write().await;
                    let conversation_depth = self.get_conversation_depth(&user_id).await;
                    let last_success = self.get_last_interaction_success(&user_id).await;

                    policy_lock.decide_routing(
                        from,
                        &message.content,
                        conversation_depth,
                        last_success,
                    )
                } else {
                    from.to_string() // Stay with current
                }
            }

            #[cfg(not(feature = "rl"))]
            {
                // Fallback: use preference predictor
                if let (Some(predictor), Some(ref uid)) = (&self.preference_predictor, &user_id) {
                    let registry = self.registry.read().await;
                    let agents: Vec<String> = registry.iter().map(|(name, _)| name.clone()).collect();
                    drop(registry);

                    predictor.predict_best_agent(uid, &agents).await?
                        .unwrap_or_else(|| from.to_string())
                } else {
                    from.to_string()
                }
            }
        } else {
            from.to_string()
        };

        // Perform transfer if needed
        if target_agent != from {
            self.transfer(from, &target_agent, message).await
        } else {
            self.process_message(message, user_id).await
        }
    }

    /// Standard transfer with learning tracking
    pub async fn transfer(&self, from: &str, to: &str, message: Message) -> Result<Message> {
        // Validate agents exist
        {
            let registry = self.registry.read().await;
            if registry.get(from).is_none() {
                return Err(anyhow!("Source agent '{}' not found", from));
            }
            if registry.get(to).is_none() {
                return Err(anyhow!("Target agent '{}' not found", to));
            }
        }

        // Get the source agent and perform transfer
        let source_agent = {
            let registry = self.registry.read().await;
            registry.get(from).unwrap().clone()
        };

        let result = source_agent.transfer_to(to.to_string(), message).await?;

        // Update current agent
        self.set_current_agent_name(to).await?;

        Ok(result)
    }

    /// Update learning system with feedback
    pub async fn provide_feedback(&self, user_id: String, satisfaction: f64) -> Result<()> {
        if let Some(tracker) = &self.tracker {
            // Get the last interaction
            let history = tracker.get_user_history(&user_id, 1).await?;

            if let Some(mut last_interaction) = history.into_iter().next() {
                // Update with satisfaction
                last_interaction.user_satisfaction = Some(satisfaction);
                tracker.record(last_interaction.clone()).await?;

                // Update personality if enabled
                if let Some(adapter) = &self.personality_adapter {
                    let mut adapter_lock = adapter.write().await;
                    adapter_lock.update_from_feedback(
                        &last_interaction.agent_name,
                        &user_id,
                        satisfaction,
                        last_interaction.response_content.len(),
                        last_interaction.duration_ms,
                    ).await?;
                }

                // Update routing policy if enabled
                #[cfg(feature = "rl")]
                if let Some(policy) = &self.routing_policy {
                    use crate::agents::learning::routing::{RoutingState, RoutingAction};

                    let mut policy_lock = policy.write().await;
                    let state = RoutingState {
                        current_agent: last_interaction.agent_name.clone(),
                        user_intent: RoutingState::classify_intent(&last_interaction.message_content),
                        conversation_depth: 1,
                        last_success: satisfaction > 0.6,
                    };

                    let action = if last_interaction.transfer_occurred {
                        if let Some(ref target) = last_interaction.target_agent {
                            match target.as_str() {
                                "git" => RoutingAction::TransferToGit,
                                "haiku" => RoutingAction::TransferToHaiku,
                                "project" => RoutingAction::TransferToProject,
                                "greeter" => RoutingAction::TransferToGreeter,
                                _ => RoutingAction::StayWithCurrent,
                            }
                        } else {
                            RoutingAction::StayWithCurrent
                        }
                    } else {
                        RoutingAction::StayWithCurrent
                    };

                    policy_lock.update_from_outcome(
                        state,
                        action,
                        satisfaction > 0.6,
                        Some(satisfaction),
                    ).await;
                }
            }
        }

        Ok(())
    }

    /// Train routing policy from historical data
    #[cfg(feature = "rl")]
    pub async fn train_routing_policy(&self, user_id: &str, episodes: usize) -> Result<()> {
        if let Some(policy) = &self.routing_policy {
            let mut policy_lock = policy.write().await;
            policy_lock.train_from_history(user_id, episodes).await?;
        }
        Ok(())
    }

    /// Save learned models
    #[cfg(feature = "rl")]
    pub async fn save_models(&self, base_path: &str) -> Result<()> {
        if let Some(policy) = &self.routing_policy {
            let policy_lock = policy.read().await;
            policy_lock.save_model(&format!("{}/routing_policy.json", base_path)).await?;
        }
        Ok(())
    }

    /// Load learned models
    #[cfg(feature = "rl")]
    pub async fn load_models(&mut self, base_path: &str) -> Result<()> {
        if let Some(tracker) = &self.tracker {
            let policy = AgentRoutingPolicy::load_model(
                tracker.clone(),
                &format!("{}/routing_policy.json", base_path)
            ).await?;
            self.routing_policy = Some(Arc::new(RwLock::new(policy)));
        }
        Ok(())
    }

    // Helper methods
    async fn get_conversation_depth(&self, user_id: &Option<String>) -> i32 {
        if let (Some(tracker), Some(uid)) = (&self.tracker, user_id) {
            tracker.get_user_history(uid, 10).await
                .map(|h| h.len() as i32)
                .unwrap_or(0)
        } else {
            0
        }
    }

    async fn get_last_interaction_success(&self, user_id: &Option<String>) -> bool {
        if let (Some(tracker), Some(uid)) = (&self.tracker, user_id) {
            tracker.get_user_history(uid, 1).await
                .ok()
                .and_then(|h| h.into_iter().next())
                .map(|i| i.success)
                .unwrap_or(true)
        } else {
            true
        }
    }

    async fn get_agent(&self, name: &str) -> Result<Arc<Box<dyn Agent + Send + Sync>>> {
        let registry = self.registry.read().await;
        registry.get(name)
            .map(|wrapper| Arc::new(Box::new(wrapper.clone()) as Box<dyn Agent + Send + Sync>))
            .ok_or_else(|| anyhow!("Agent '{}' not found", name))
    }

    async fn get_current_agent_name(&self) -> Result<String> {
        let registry = self.registry.read().await;
        registry.get_current_agent()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No current agent set"))
    }

    async fn set_current_agent_name(&self, target: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        if registry.get(target).is_some() {
            registry.set_current_agent(target.to_string());
            Ok(())
        } else {
            Err(anyhow!("Target agent '{}' not found", target))
        }
    }

    /// Get statistics about learning
    pub async fn get_learning_stats(&self, user_id: &str) -> Result<LearningStats> {
        if let Some(tracker) = &self.tracker {
            let history = tracker.get_user_history(user_id, 100).await?;

            let total_interactions = history.len();
            let successful_interactions = history.iter().filter(|i| i.success).count();
            let avg_satisfaction = if history.is_empty() {
                0.0
            } else {
                history.iter()
                    .filter_map(|i| i.user_satisfaction)
                    .sum::<f64>() / history.len() as f64
            };

            let transfers = history.iter().filter(|i| i.transfer_occurred).count();

            Ok(LearningStats {
                total_interactions,
                successful_interactions,
                success_rate: successful_interactions as f64 / total_interactions.max(1) as f64,
                avg_satisfaction,
                total_transfers: transfers,
            })
        } else {
            Ok(LearningStats::default())
        }
    }
}

#[derive(Debug, Clone)]
pub struct LearningStats {
    pub total_interactions: usize,
    pub successful_interactions: usize,
    pub success_rate: f64,
    pub avg_satisfaction: f64,
    pub total_transfers: usize,
}

impl Default for LearningStats {
    fn default() -> Self {
        Self {
            total_interactions: 0,
            successful_interactions: 0,
            success_rate: 0.0,
            avg_satisfaction: 0.0,
            total_transfers: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfig;
    use crate::agents::greeter::GreeterAgent;

    #[tokio::test]
    async fn test_learning_transfer_service() -> Result<()> {
        // Create registry
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "greeter".to_string(),
            public_description: "Test greeter".to_string(),
            instructions: "Test".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });
        registry.register("greeter".to_string(), Box::new(agent)).await?;
        let registry = Arc::new(RwLock::new(registry));

        // Create service without learning (should work)
        let service = LearningTransferService::new(registry.clone(), LearningConfig::default()).await?;

        // Process a message
        let response = service.process_message(
            Message::new("Hello".to_string()),
            None,
        ).await?;

        assert!(!response.content.is_empty());

        Ok(())
    }
}
