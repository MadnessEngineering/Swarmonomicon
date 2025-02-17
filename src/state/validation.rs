use std::collections::{HashMap, HashSet};
use serde_json::Value;
use anyhow::{Result, anyhow};
use super::{PersistedState, StateValidator};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub pattern: Regex,
    pub error_message: String,
}

impl ValidationRule {
    pub fn new(pattern: &str, error_message: &str) -> Result<Self> {
        Ok(Self {
            pattern: Regex::new(pattern)?,
            error_message: error_message.to_string(),
        })
    }

    pub fn validate(&self, value: &str) -> bool {
        self.pattern.is_match(value)
    }
}

pub struct StateValidationConfig {
    state_rules: HashMap<String, Vec<ValidationRule>>,
    transition_rules: HashMap<(String, String), Vec<ValidationRule>>,
    data_rules: HashMap<String, Vec<ValidationRule>>,
    valid_states: HashSet<String>,
    valid_transitions: HashSet<(String, String)>,
}

impl StateValidationConfig {
    pub fn new() -> Self {
        Self {
            state_rules: HashMap::new(),
            transition_rules: HashMap::new(),
            data_rules: HashMap::new(),
            valid_states: HashSet::new(),
            valid_transitions: HashSet::new(),
        }
    }

    pub fn add_state(&mut self, state_name: &str) {
        self.valid_states.insert(state_name.to_string());
    }

    pub fn add_transition(&mut self, from: &str, to: &str) {
        self.valid_transitions.insert((from.to_string(), to.to_string()));
    }

    pub fn add_state_rule(&mut self, state: &str, pattern: &str, error_message: &str) -> Result<()> {
        let rule = ValidationRule::new(pattern, error_message)?;
        self.state_rules
            .entry(state.to_string())
            .or_insert_with(Vec::new)
            .push(rule);
        Ok(())
    }

    pub fn add_transition_rule(
        &mut self,
        from: &str,
        to: &str,
        pattern: &str,
        error_message: &str,
    ) -> Result<()> {
        let rule = ValidationRule::new(pattern, error_message)?;
        self.transition_rules
            .entry((from.to_string(), to.to_string()))
            .or_insert_with(Vec::new)
            .push(rule);
        Ok(())
    }

    pub fn add_data_rule(&mut self, field: &str, pattern: &str, error_message: &str) -> Result<()> {
        let rule = ValidationRule::new(pattern, error_message)?;
        self.data_rules
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(rule);
        Ok(())
    }
}

pub struct StateValidatorImpl {
    config: StateValidationConfig,
}

impl StateValidatorImpl {
    pub fn new(config: StateValidationConfig) -> Self {
        Self { config }
    }

    fn validate_state_rules(&self, state_name: &str, state: &PersistedState) -> Result<()> {
        if !self.config.valid_states.contains(state_name) {
            return Err(anyhow!("Invalid state: {}", state_name));
        }

        if let Some(rules) = self.config.state_rules.get(state_name) {
            for rule in rules {
                if !rule.validate(&state.state_name) {
                    return Err(anyhow!("{}", rule.error_message));
                }
            }
        }
        Ok(())
    }

    fn validate_transition_rules(&self, from: &str, to: &str) -> Result<()> {
        if !self.config.valid_transitions.contains(&(from.to_string(), to.to_string())) {
            return Err(anyhow!("Invalid transition from {} to {}", from, to));
        }

        if let Some(rules) = self.config.transition_rules.get(&(from.to_string(), to.to_string())) {
            for rule in rules {
                if !rule.validate(to) {
                    return Err(anyhow!("{}", rule.error_message));
                }
            }
        }
        Ok(())
    }

    fn validate_data_rules(&self, data: &Value) -> Result<()> {
        if let Value::Object(map) = data {
            for (field, value) in map {
                if let Some(rules) = self.config.data_rules.get(field) {
                    if let Value::String(s) = value {
                        for rule in rules {
                            if !rule.validate(s) {
                                return Err(anyhow!("{}: {}", field, rule.error_message));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl StateValidator for StateValidatorImpl {
    fn validate_state(&self, state: &PersistedState) -> Result<()> {
        self.validate_state_rules(&state.state_name, state)
    }

    fn validate_transition(&self, from: &str, to: &str) -> Result<()> {
        self.validate_transition_rules(from, to)
    }

    fn validate_data(&self, state_data: &Value) -> Result<()> {
        self.validate_data_rules(state_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_config() -> Result<StateValidationConfig> {
        let mut config = StateValidationConfig::new();

        // Add valid states
        config.add_state("initial");
        config.add_state("processing");
        config.add_state("completed");

        // Add valid transitions
        config.add_transition("initial", "processing");
        config.add_transition("processing", "completed");

        // Add state rules
        config.add_state_rule(
            "initial",
            "^initial$",
            "Initial state name must match exactly",
        )?;

        // Add transition rules
        config.add_transition_rule(
            "initial",
            "processing",
            "^processing$",
            "Processing state name must match exactly",
        )?;

        // Add data rules
        config.add_data_rule(
            "user_id",
            "^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
            "Invalid UUID format",
        )?;

        Ok(config)
    }

    #[test]
    fn test_state_validation() -> Result<()> {
        let config = create_test_config()?;
        let validator = StateValidatorImpl::new(config);

        // Test valid state
        let valid_state = PersistedState {
            agent_id: "test_agent".to_string(),
            state_name: "initial".to_string(),
            state_data: None,
            conversation_context: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            metadata: HashMap::new(),
        };
        assert!(validator.validate_state(&valid_state).is_ok());

        // Test invalid state
        let invalid_state = PersistedState {
            state_name: "invalid".to_string(),
            ..valid_state.clone()
        };
        assert!(validator.validate_state(&invalid_state).is_err());

        Ok(())
    }

    #[test]
    fn test_transition_validation() -> Result<()> {
        let config = create_test_config()?;
        let validator = StateValidatorImpl::new(config);

        // Test valid transition
        assert!(validator.validate_transition("initial", "processing").is_ok());

        // Test invalid transition
        assert!(validator.validate_transition("initial", "completed").is_err());
        assert!(validator.validate_transition("completed", "initial").is_err());

        Ok(())
    }

    #[test]
    fn test_data_validation() -> Result<()> {
        let config = create_test_config()?;
        let validator = StateValidatorImpl::new(config);

        // Test valid data
        let valid_data = serde_json::json!({
            "user_id": "550e8400-e29b-41d4-a716-446655440000"
        });
        assert!(validator.validate_data(&valid_data).is_ok());

        // Test invalid data
        let invalid_data = serde_json::json!({
            "user_id": "invalid-uuid"
        });
        assert!(validator.validate_data(&invalid_data).is_err());

        Ok(())
    }
} 
