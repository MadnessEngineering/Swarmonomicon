use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::types::{AgentConfig, Tool, ToolParameter};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSet {
    pub name: String,
    pub description: String,
    pub agents: Vec<AgentConfig>,
}

pub struct ConfigManager {
    agent_sets: HashMap<String, AgentSet>,
    tool_templates: HashMap<String, Tool>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            agent_sets: HashMap::new(),
            tool_templates: HashMap::new(),
        }
    }

    pub fn register_agent_set(&mut self, agent_set: AgentSet) {
        self.agent_sets.insert(agent_set.name.clone(), agent_set);
    }

    pub fn register_tool_template(&mut self, name: String, tool: Tool) {
        self.tool_templates.insert(name, tool);
    }

    pub fn get_agent_set(&self, name: &str) -> Option<&AgentSet> {
        self.agent_sets.get(name)
    }

    pub fn get_tool_template(&self, name: &str) -> Option<&Tool> {
        self.tool_templates.get(name)
    }

    pub fn inject_transfer_tools(&mut self, agent_set: &mut AgentSet) -> Result<()> {
        let transfer_tool = get_transfer_tool();

        for agent in &mut agent_set.agents {
            if !agent.downstream_agents.is_empty() {
                agent.tools.push(transfer_tool.clone());
            }
        }

        Ok(())
    }
}

pub fn get_transfer_tool() -> Tool {
    Tool {
        name: "agent_transfer".to_string(),
        description: "Transfer control to another agent".to_string(),
        parameters: {
            let mut params: HashMap<String, ToolParameter> = HashMap::new();
            params.insert(
                "target_agent".to_string(),
                ToolParameter {
                    type_name: "string".to_string(),
                    description: Some("Name of the agent to transfer to".to_string()),
                    enum_values: None,
                    pattern: None,
                    properties: None,
                    required: None,
                    additional_properties: None,
                    items: None,
                }
            );
            params
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_agent_set() -> AgentSet {
        AgentSet {
            name: "test_set".to_string(),
            description: "Test agent set".to_string(),
            agents: vec![
                AgentConfig {
                    name: "greeter".to_string(),
                    public_description: "Greets users".to_string(),
                    instructions: "Greet the user".to_string(),
                    tools: vec![],
                    downstream_agents: vec!["haiku".to_string()],
                    personality: None,
                    state_machine: None,
                },
            ],
        }
    }

    #[test]
    fn test_config_manager() {
        let mut manager = ConfigManager::new();
        let agent_set = create_test_agent_set();
        manager.register_agent_set(agent_set.clone());

        let retrieved = manager.get_agent_set("test_set").unwrap();
        assert_eq!(retrieved.name, "test_set");
        assert_eq!(retrieved.agents.len(), 1);
    }

    #[test]
    fn test_inject_transfer_tools() {
        let mut manager = ConfigManager::new();
        let mut agent_set = create_test_agent_set();

        manager.inject_transfer_tools(&mut agent_set).unwrap();

        let agent = &agent_set.agents[0];
        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.tools[0].name, "agent_transfer");
    }
}
