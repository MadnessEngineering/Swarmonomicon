use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use mongodb::{Client, Collection, bson::doc};
use anyhow::Result;
use futures::TryStreamExt;

/// Represents a single user interaction with an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    pub id: String,
    pub user_id: String,
    pub agent_name: String,
    pub message_content: String,
    pub response_content: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: i64,
    pub success: bool,
    pub transfer_occurred: bool,
    pub target_agent: Option<String>,
    pub user_satisfaction: Option<f64>, // 0.0-1.0, optional feedback
    pub context: HashMap<String, String>,
}

impl UserInteraction {
    pub fn new(
        user_id: String,
        agent_name: String,
        message: String,
        response: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            agent_name,
            message_content: message,
            response_content: response,
            timestamp: Utc::now(),
            duration_ms: 0,
            success: true,
            transfer_occurred: false,
            target_agent: None,
            user_satisfaction: None,
            context: HashMap::new(),
        }
    }

    pub fn with_duration(mut self, duration_ms: i64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_transfer(mut self, target: String) -> Self {
        self.transfer_occurred = true;
        self.target_agent = Some(target);
        self
    }

    pub fn with_satisfaction(mut self, score: f64) -> Self {
        self.user_satisfaction = Some(score.clamp(0.0, 1.0));
        self
    }

    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
}

/// Tracks and stores user interactions for learning
#[derive(Clone)]
pub struct InteractionTracker {
    interactions: Collection<UserInteraction>,
}

impl InteractionTracker {
    pub async fn new(client: &Client) -> Result<Self> {
        let db = client.database("swarmonomicon");
        let interactions = db.collection("user_interactions");

        // Create indexes for efficient querying
        let user_index = mongodb::IndexModel::builder()
            .keys(doc! { "user_id": 1, "timestamp": -1 })
            .build();
        interactions.create_index(user_index, None).await?;

        let agent_index = mongodb::IndexModel::builder()
            .keys(doc! { "agent_name": 1, "timestamp": -1 })
            .build();
        interactions.create_index(agent_index, None).await?;

        Ok(Self { interactions })
    }

    /// Record a new user interaction
    pub async fn record(&self, interaction: UserInteraction) -> Result<()> {
        self.interactions.insert_one(interaction, None).await?;
        Ok(())
    }

    /// Get recent interactions for a specific user
    pub async fn get_user_history(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<UserInteraction>> {
        let filter = doc! { "user_id": user_id };
        let options = mongodb::options::FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = self.interactions.find(filter, options).await?;
        let mut history = Vec::new();
        while let Some(interaction) = cursor.try_next().await? {
            history.push(interaction);
        }
        Ok(history)
    }

    /// Get interactions with a specific agent
    pub async fn get_agent_interactions(
        &self,
        agent_name: &str,
        limit: i64,
    ) -> Result<Vec<UserInteraction>> {
        let filter = doc! { "agent_name": agent_name };
        let options = mongodb::options::FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = self.interactions.find(filter, options).await?;
        let mut history = Vec::new();
        while let Some(interaction) = cursor.try_next().await? {
            history.push(interaction);
        }
        Ok(history)
    }

    /// Get transfer patterns for learning routing
    pub async fn get_transfer_patterns(&self, user_id: &str) -> Result<Vec<(String, String)>> {
        let filter = doc! {
            "user_id": user_id,
            "transfer_occurred": true
        };
        let options = mongodb::options::FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(100)
            .build();

        let mut cursor = self.interactions.find(filter, options).await?;
        let mut patterns = Vec::new();
        while let Some(interaction) = cursor.try_next().await? {
            if let Some(target) = interaction.target_agent {
                patterns.push((interaction.agent_name, target));
            }
        }
        Ok(patterns)
    }

    /// Calculate success rate for a user-agent pair
    pub async fn calculate_success_rate(
        &self,
        user_id: &str,
        agent_name: &str,
    ) -> Result<f64> {
        let filter = doc! {
            "user_id": user_id,
            "agent_name": agent_name
        };

        let total = self.interactions.count_documents(filter.clone(), None).await? as f64;
        if total == 0.0 {
            return Ok(0.5); // Default neutral score
        }

        let success_filter = doc! {
            "user_id": user_id,
            "agent_name": agent_name,
            "success": true
        };
        let successful = self.interactions.count_documents(success_filter, None).await? as f64;

        Ok(successful / total)
    }

    /// Get average satisfaction score for user-agent pair
    pub async fn get_average_satisfaction(
        &self,
        user_id: &str,
        agent_name: &str,
    ) -> Result<Option<f64>> {
        let filter = doc! {
            "user_id": user_id,
            "agent_name": agent_name,
            "user_satisfaction": { "$exists": true, "$ne": null }
        };

        let mut cursor = self.interactions.find(filter, None).await?;
        let mut scores = Vec::new();
        while let Some(interaction) = cursor.try_next().await? {
            if let Some(score) = interaction.user_satisfaction {
                scores.push(score);
            }
        }

        if scores.is_empty() {
            Ok(None)
        } else {
            Ok(Some(scores.iter().sum::<f64>() / scores.len() as f64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interaction_tracking() -> Result<()> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let tracker = InteractionTracker::new(&client).await?;

        // Create test interaction
        let interaction = UserInteraction::new(
            "test_user".to_string(),
            "greeter".to_string(),
            "Hello".to_string(),
            "Hi there!".to_string(),
        )
        .with_duration(150)
        .with_satisfaction(0.9);

        // Record it
        tracker.record(interaction).await?;

        // Retrieve history
        let history = tracker.get_user_history("test_user", 10).await?;
        assert!(!history.is_empty());

        // Check success rate
        let success_rate = tracker.calculate_success_rate("test_user", "greeter").await?;
        assert!(success_rate > 0.0);

        Ok(())
    }
}
