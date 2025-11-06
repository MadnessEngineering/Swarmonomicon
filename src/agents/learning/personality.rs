use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use super::interaction::{InteractionTracker, UserInteraction};
use super::preference::{PreferencePredictor, PreferenceCategory};

/// Personality trait that can be adjusted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PersonalityTrait {
    Friendliness,   // 0.0 = formal, 1.0 = very friendly
    Verbosity,      // 0.0 = concise, 1.0 = detailed
    Proactivity,    // 0.0 = reactive, 1.0 = proactive
    Humor,          // 0.0 = serious, 1.0 = humorous
    Technical,      // 0.0 = simple, 1.0 = technical
    Patience,       // 0.0 = quick, 1.0 = patient
}

/// Represents an agent's personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub agent_name: String,
    pub traits: HashMap<PersonalityTrait, f64>, // 0.0-1.0 for each trait
    pub user_specific: Option<HashMap<String, HashMap<PersonalityTrait, f64>>>,
}

impl PersonalityProfile {
    pub fn new(agent_name: String) -> Self {
        let mut traits = HashMap::new();
        // Default neutral personality
        traits.insert(PersonalityTrait::Friendliness, 0.7);
        traits.insert(PersonalityTrait::Verbosity, 0.5);
        traits.insert(PersonalityTrait::Proactivity, 0.5);
        traits.insert(PersonalityTrait::Humor, 0.3);
        traits.insert(PersonalityTrait::Technical, 0.5);
        traits.insert(PersonalityTrait::Patience, 0.7);

        Self {
            agent_name,
            traits,
            user_specific: None,
        }
    }

    /// Get trait value for a specific user, falling back to default
    pub fn get_trait(&self, trait_type: &PersonalityTrait, user_id: Option<&str>) -> f64 {
        if let Some(user_id) = user_id {
            if let Some(ref user_profiles) = self.user_specific {
                if let Some(user_traits) = user_profiles.get(user_id) {
                    if let Some(&value) = user_traits.get(trait_type) {
                        return value;
                    }
                }
            }
        }
        self.traits.get(trait_type).copied().unwrap_or(0.5)
    }

    /// Set trait value for a specific user
    pub fn set_user_trait(&mut self, user_id: String, trait_type: PersonalityTrait, value: f64) {
        let clamped_value = value.clamp(0.0, 1.0);

        if self.user_specific.is_none() {
            self.user_specific = Some(HashMap::new());
        }

        self.user_specific
            .as_mut()
            .unwrap()
            .entry(user_id)
            .or_insert_with(HashMap::new)
            .insert(trait_type, clamped_value);
    }

    /// Adjust trait based on feedback
    pub fn adjust_trait(&mut self, user_id: Option<String>, trait_type: PersonalityTrait, delta: f64) {
        if let Some(user_id) = user_id {
            let current = self.get_trait(&trait_type, Some(&user_id));
            let new_value = (current + delta).clamp(0.0, 1.0);
            self.set_user_trait(user_id, trait_type, new_value);
        } else {
            // Adjust global trait
            let current = self.traits.get(&trait_type).copied().unwrap_or(0.5);
            let new_value = (current + delta).clamp(0.0, 1.0);
            self.traits.insert(trait_type, new_value);
        }
    }

    /// Generate prompt modifier based on personality
    pub fn get_prompt_modifier(&self, user_id: Option<&str>) -> String {
        let friendliness = self.get_trait(&PersonalityTrait::Friendliness, user_id);
        let verbosity = self.get_trait(&PersonalityTrait::Verbosity, user_id);
        let humor = self.get_trait(&PersonalityTrait::Humor, user_id);
        let technical = self.get_trait(&PersonalityTrait::Technical, user_id);

        let mut modifiers = Vec::new();

        // Friendliness
        if friendliness > 0.7 {
            modifiers.push("Be warm and friendly");
        } else if friendliness < 0.3 {
            modifiers.push("Be formal and professional");
        }

        // Verbosity
        if verbosity > 0.7 {
            modifiers.push("Provide detailed, comprehensive responses");
        } else if verbosity < 0.3 {
            modifiers.push("Be concise and to-the-point");
        }

        // Humor
        if humor > 0.6 {
            modifiers.push("Use appropriate humor when suitable");
        } else if humor < 0.2 {
            modifiers.push("Maintain a serious, professional tone");
        }

        // Technical
        if technical > 0.7 {
            modifiers.push("Use technical terminology and detailed explanations");
        } else if technical < 0.3 {
            modifiers.push("Use simple, accessible language");
        }

        if modifiers.is_empty() {
            String::new()
        } else {
            format!("\n\nPersonality guidelines: {}", modifiers.join(". "))
        }
    }
}

/// Adapts agent personality based on user interactions
pub struct PersonalityAdapter {
    tracker: InteractionTracker,
    predictor: PreferencePredictor,
    profiles: HashMap<String, PersonalityProfile>,
}

impl PersonalityAdapter {
    pub fn new(tracker: InteractionTracker) -> Self {
        let predictor = PreferencePredictor::new(tracker.clone());

        Self {
            tracker,
            predictor,
            profiles: HashMap::new(),
        }
    }

    /// Initialize or get personality profile for an agent
    pub fn get_profile(&mut self, agent_name: &str) -> &mut PersonalityProfile {
        self.profiles
            .entry(agent_name.to_string())
            .or_insert_with(|| PersonalityProfile::new(agent_name.to_string()))
    }

    /// Adapt personality based on user preferences
    pub async fn adapt_to_user(&mut self, agent_name: &str, user_id: &str) -> Result<()> {
        // Get user preferences
        let profile = self.predictor.get_user_profile(user_id).await?;

        let personality = self.get_profile(agent_name);

        // Adjust verbosity based on response style preference
        if let Some(style_pref) = profile.get(&PreferenceCategory::ResponseStyle) {
            if style_pref.confidence > 0.3 {
                personality.set_user_trait(
                    user_id.to_string(),
                    PersonalityTrait::Verbosity,
                    style_pref.value,
                );
            }
        }

        // Adjust patience based on interaction speed preference
        if let Some(speed_pref) = profile.get(&PreferenceCategory::InteractionSpeed) {
            if speed_pref.confidence > 0.3 {
                // High speed preference = low patience (quick responses)
                personality.set_user_trait(
                    user_id.to_string(),
                    PersonalityTrait::Patience,
                    1.0 - speed_pref.value,
                );
            }
        }

        Ok(())
    }

    /// Update personality based on interaction feedback
    pub async fn update_from_feedback(
        &mut self,
        agent_name: &str,
        user_id: &str,
        satisfaction: f64,
        response_length: usize,
        duration_ms: i64,
    ) -> Result<()> {
        let personality = self.get_profile(agent_name);

        // If satisfaction is low, adjust personality
        if satisfaction < 0.5 {
            // Negative feedback - adjust away from current settings
            let verbosity = personality.get_trait(&PersonalityTrait::Verbosity, Some(user_id));
            let friendliness = personality.get_trait(&PersonalityTrait::Friendliness, Some(user_id));

            // If response was long and user unsatisfied, reduce verbosity
            if response_length > 500 && verbosity > 0.5 {
                personality.adjust_trait(
                    Some(user_id.to_string()),
                    PersonalityTrait::Verbosity,
                    -0.1,
                );
            }

            // If very formal and user unsatisfied, try being more friendly
            if friendliness < 0.4 {
                personality.adjust_trait(
                    Some(user_id.to_string()),
                    PersonalityTrait::Friendliness,
                    0.1,
                );
            }
        } else if satisfaction > 0.8 {
            // Positive feedback - reinforce current settings slightly
            let verbosity = personality.get_trait(&PersonalityTrait::Verbosity, Some(user_id));

            if response_length > 500 && verbosity > 0.5 {
                // User likes detailed responses
                personality.adjust_trait(
                    Some(user_id.to_string()),
                    PersonalityTrait::Verbosity,
                    0.05,
                );
            }
        }

        Ok(())
    }

    /// Get personality-adjusted system prompt
    pub fn get_adjusted_prompt(
        &mut self,
        agent_name: &str,
        base_prompt: &str,
        user_id: Option<&str>,
    ) -> String {
        let profile = self.get_profile(agent_name);
        let modifier = profile.get_prompt_modifier(user_id);

        if modifier.is_empty() {
            base_prompt.to_string()
        } else {
            format!("{}{}", base_prompt, modifier)
        }
    }

    /// Load personality profiles from persistence
    pub fn load_profiles(&mut self, profiles: HashMap<String, PersonalityProfile>) {
        self.profiles = profiles;
    }

    /// Get all profiles for saving
    pub fn get_all_profiles(&self) -> &HashMap<String, PersonalityProfile> {
        &self.profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::Client;

    #[tokio::test]
    async fn test_personality_adaptation() -> Result<()> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let tracker = InteractionTracker::new(&client).await?;
        let mut adapter = PersonalityAdapter::new(tracker);

        // Get profile for agent
        let profile = adapter.get_profile("greeter");
        assert_eq!(profile.agent_name, "greeter");

        // Test trait adjustment
        profile.set_user_trait(
            "test_user".to_string(),
            PersonalityTrait::Friendliness,
            0.9,
        );

        assert_eq!(
            profile.get_trait(&PersonalityTrait::Friendliness, Some("test_user")),
            0.9
        );

        // Test prompt modifier generation
        let modifier = profile.get_prompt_modifier(Some("test_user"));
        assert!(modifier.contains("friendly") || modifier.contains("warm"));

        Ok(())
    }
}
