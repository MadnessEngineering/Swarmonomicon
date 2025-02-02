use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, Result, ToolCall};
use std::sync::Arc;

pub struct BalenaWrapperAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
    inner: Arc<dyn Agent + Send + Sync>,
}

impl BalenaWrapperAgent {
    pub fn new(config: AgentConfig) -> Self {
        // Setup the state machine as before...
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("awaiting_command".to_string(), crate::types::State {
                    prompt: "🛸 Fleet Commander ready. What IoT operations shall we initiate today?".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("command_received".to_string(), "executing".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("executing".to_string(), crate::types::State {
                    prompt: "⚡ Executing fleet command... Stand by for quantum entanglement.".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("complete".to_string(), "awaiting_command".to_string());
                        transitions
                    },
                    validation: None,
                });
                states
            },
            initial_state: "awaiting_command".to_string(),
        });

        Self {
            config,
            state_manager: AgentStateManager::new(state_machine),
            inner: Arc::new(Self {
                config,
                state_manager: AgentStateManager::new(state_machine),
            }),
        }
    }


    fn create_response(&self, content: String) -> Message {
        let current_state = self.state_manager.get_current_state_name();
        let metadata = MessageMetadata::new(self.config.name.clone())
            .with_state(current_state.unwrap_or("awaiting_command").to_string())
            .with_personality(vec![
                "fleet_commander".to_string(),
                "precise".to_string(),
                "iot_focused".to_string(),
                "system_oriented".to_string(),
                "deployment_expert".to_string(),
            ]);

        Message {
            content,
            metadata,
            parameters: {
                let mut params = HashMap::new();
                params.insert("style".to_string(), "fleet_commander".to_string());
                params.insert("domain".to_string(), "iot_operations".to_string());
                params
            },
            tool_calls: None,
            confidence: Some(1.0),
        }
    }

    fn format_fleet_response(&self, content: String) -> Message {
        let prefix = match content.to_lowercase() {
            s if s.contains("error") => "🚨 Fleet Alert: ",
            s if s.contains("success") => "✅ Operation Complete: ",
            s if s.contains("push") => "🚀 Deployment Status: ",
            s if s.contains("device") => "📱 Device Update: ",
            _ => "🛸 Fleet Command: ",
        };

        self.create_response(format!("{}{}", prefix, content))
    }

    fn get_help_message(&self) -> Message {
        self.format_fleet_response(
            "Welcome to the IoT Fleet Command Center! Available operations:\n\
             - 'devices': Scan for connected fleet units\n\
             - 'push <app>': Deploy updates to target application\n\
             - 'logs <device>': Monitor quantum transmissions from device\n\
             - 'ssh <device>': Establish secure neural link to device\n\
             - 'status': Monitor fleet-wide operational status\n\
             - 'wifi <device> <ssid> <psk>': Configure device network matrix\n\
             - 'scan': Initiate fleet-wide diagnostics".to_string()
        )
    }

    async fn execute_balena_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("balena")
            .args(args)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Ok(format!("Error: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}


#[async_trait]
impl Agent for BalenaWrapperAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        // Use the existing logic to handle commands...
        // ...
        Ok(self.create_response("Response from BalenaWrapperAgent".to_string()))
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, _tool_call: ToolCall) -> Result<Message> {
        Ok(self.create_response(
            "Direct tool interface not available. Please use fleet command protocols."
                .to_string(),
        ))
    }

    async fn get_current_state(&self) -> Result<Option<crate::types::State>> {
        Ok(self.state_manager.get_current_state().cloned())
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}
