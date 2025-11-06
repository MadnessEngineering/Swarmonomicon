use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use super::interaction::InteractionTracker;

/// User preference categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PreferenceCategory {
    AgentChoice,      // Preferred agents
    ResponseStyle,    // Detailed vs concise
    InteractionSpeed, // Fast vs thorough
    Formality,        // Casual vs formal
    ProactivityLevel, // Reactive vs proactive
}

/// Represents a learned user preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub user_id: String,
    pub category: PreferenceCategory,
    pub value: f64, // Normalized 0.0-1.0
    pub confidence: f64, // How confident we are (based on data points)
    pub agent_specific: Option<HashMap<String, f64>>, // Per-agent preferences
}

/// Predicts and learns user preferences from interaction history
pub struct PreferencePredictor {
    tracker: InteractionTracker,
}

impl PreferencePredictor {
    pub fn new(tracker: InteractionTracker) -> Self {
        Self { tracker }
    }

    /// Learn agent preferences from interaction history
    pub async fn learn_agent_preferences(&self, user_id: &str) -> Result<UserPreference> {
        let history = self.tracker.get_user_history(user_id, 100).await?;

        if history.is_empty() {
            return Ok(UserPreference {
                user_id: user_id.to_string(),
                category: PreferenceCategory::AgentChoice,
                value: 0.5,
                confidence: 0.0,
                agent_specific: None,
            });
        }

        // Count interactions and calculate success rates per agent
        let mut agent_scores: HashMap<String, (f64, i32)> = HashMap::new();

        for interaction in &history {
            let success_score = if interaction.success {
                interaction.user_satisfaction.unwrap_or(0.7)
            } else {
                0.3
            };

            let (total_score, count) = agent_scores
                .entry(interaction.agent_name.clone())
                .or_insert((0.0, 0));

            *total_score += success_score;
            *count += 1;
        }

        // Calculate average scores per agent
        let mut agent_preferences = HashMap::new();
        for (agent, (total, count)) in agent_scores {
            agent_preferences.insert(agent, total / count as f64);
        }

        // Overall preference is the variance in agent scores
        // Higher variance = stronger preferences
        let scores: Vec<f64> = agent_preferences.values().copied().collect();
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance: f64 = scores.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f64>() / scores.len() as f64;

        Ok(UserPreference {
            user_id: user_id.to_string(),
            category: PreferenceCategory::AgentChoice,
            value: variance.sqrt(), // Std dev as preference strength
            confidence: (history.len() as f64 / 100.0).min(1.0),
            agent_specific: Some(agent_preferences),
        })
    }

    /// Learn response style preference (detailed vs concise)
    pub async fn learn_response_style(&self, user_id: &str) -> Result<UserPreference> {
        let history = self.tracker.get_user_history(user_id, 50).await?;

        if history.is_empty() {
            return Ok(self.default_preference(user_id, PreferenceCategory::ResponseStyle));
        }

        // Analyze message lengths and satisfaction
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for interaction in &history {
            let response_length = interaction.response_content.len() as f64;
            let normalized_length = (response_length / 1000.0).min(1.0); // Normalize to 0-1

            let weight = interaction.user_satisfaction.unwrap_or(0.5);
            weighted_sum += normalized_length * weight;
            total_weight += weight;
        }

        let preference_value = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.5 // Default to medium
        };

        Ok(UserPreference {
            user_id: user_id.to_string(),
            category: PreferenceCategory::ResponseStyle,
            value: preference_value,
            confidence: (history.len() as f64 / 50.0).min(1.0),
            agent_specific: None,
        })
    }

    /// Learn interaction speed preference
    pub async fn learn_interaction_speed(&self, user_id: &str) -> Result<UserPreference> {
        let history = self.tracker.get_user_history(user_id, 50).await?;

        if history.is_empty() {
            return Ok(self.default_preference(user_id, PreferenceCategory::InteractionSpeed));
        }

        // Analyze response times vs satisfaction
        let mut fast_score = 0.0;
        let mut slow_score = 0.0;
        let mut fast_count = 0;
        let mut slow_count = 0;

        for interaction in &history {
            let satisfaction = interaction.user_satisfaction.unwrap_or(0.5);

            if interaction.duration_ms < 500 {
                fast_score += satisfaction;
                fast_count += 1;
            } else {
                slow_score += satisfaction;
                slow_count += 1;
            }
        }

        // Preference toward fast responses (0.0) or slow/thorough (1.0)
        let fast_avg = if fast_count > 0 { fast_score / fast_count as f64 } else { 0.5 };
        let slow_avg = if slow_count > 0 { slow_score / slow_count as f64 } else { 0.5 };

        let preference_value = if fast_avg + slow_avg > 0.0 {
            slow_avg / (fast_avg + slow_avg)
        } else {
            0.5
        };

        Ok(UserPreference {
            user_id: user_id.to_string(),
            category: PreferenceCategory::InteractionSpeed,
            value: preference_value,
            confidence: (history.len() as f64 / 50.0).min(1.0),
            agent_specific: None,
        })
    }

    /// Get all preferences for a user
    pub async fn get_user_profile(&self, user_id: &str) -> Result<HashMap<PreferenceCategory, UserPreference>> {
        let mut profile = HashMap::new();

        profile.insert(
            PreferenceCategory::AgentChoice,
            self.learn_agent_preferences(user_id).await?,
        );

        profile.insert(
            PreferenceCategory::ResponseStyle,
            self.learn_response_style(user_id).await?,
        );

        profile.insert(
            PreferenceCategory::InteractionSpeed,
            self.learn_interaction_speed(user_id).await?,
        );

        Ok(profile)
    }

    /// Predict best agent for user based on learned preferences
    pub async fn predict_best_agent(&self, user_id: &str, available_agents: &[String]) -> Result<Option<String>> {
        let agent_prefs = self.learn_agent_preferences(user_id).await?;

        if let Some(agent_scores) = &agent_prefs.agent_specific {
            // Find agent with highest score that's available
            let mut best_agent: Option<(String, f64)> = None;

            for agent in available_agents {
                if let Some(&score) = agent_scores.get(agent) {
                    if let Some((_, best_score)) = &best_agent {
                        if score > *best_score {
                            best_agent = Some((agent.clone(), score));
                        }
                    } else {
                        best_agent = Some((agent.clone(), score));
                    }
                }
            }

            Ok(best_agent.map(|(agent, _)| agent))
        } else {
            Ok(None)
        }
    }

    fn default_preference(&self, user_id: &str, category: PreferenceCategory) -> UserPreference {
        UserPreference {
            user_id: user_id.to_string(),
            category,
            value: 0.5,
            confidence: 0.0,
            agent_specific: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::Client;
    use crate::agents::learning::interaction::UserInteraction;

    #[tokio::test]
    async fn test_preference_learning() -> Result<()> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let tracker = InteractionTracker::new(&client).await?;
        let predictor = PreferencePredictor::new(tracker);

        // Learn agent preferences
        let prefs = predictor.learn_agent_preferences("test_user").await?;
        assert_eq!(prefs.category, PreferenceCategory::AgentChoice);

        // Get full profile
        let profile = predictor.get_user_profile("test_user").await?;
        assert!(profile.contains_key(&PreferenceCategory::AgentChoice));
        assert!(profile.contains_key(&PreferenceCategory::ResponseStyle));

        Ok(())
    }
}
