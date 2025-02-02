use std::fs;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, Tool, ToolCall, State};
use crate::tools::ToolRegistry;
use crate::Result;

pub struct ProjectAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    current_state: Option<String>,
}

impl ProjectAgent {
    pub async fn new(config: AgentConfig) -> Result<Self> {
        Ok(Self {
            config,
            tools: ToolRegistry::create_default_tools().await?,
            current_state: None,
        })
    }

    fn init_python_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        // Create project structure
        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(src_dir.join(name))?;
        fs::create_dir_all(src_dir.join("tests"))?;

        // Create __init__.py files
        fs::write(src_dir.join(name).join("__init__.py"), "")?;
        fs::write(src_dir.join("tests").join("__init__.py"), "")?;

        // Create requirements.txt
        fs::write(path.join("requirements.txt"), "# Core dependencies\n")?;

        // Create setup.py
        let setup_content = format!(
            r#"from setuptools import setup, find_packages

setup(
    name="{}",
    version="0.1.0",
    packages=find_packages(where="src"),
    package_dir={{"": "src"}},
    install_requires=[],
    python_requires=">=3.8",
)"#,
            name
        );
        fs::write(path.join("setup.py"), setup_content)?;

        self.create_readme(name, description, "python", path)?;
        Ok(())
    }

    fn init_rust_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        Command::new("cargo")
            .args(["init", "--name", name])
            .current_dir(path)
            .output()?;

        self.create_readme(name, description, "rust", path)?;
        Ok(())
    }

    fn init_common_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        fs::create_dir_all(path.join("src"))?;
        fs::create_dir_all(path.join("docs"))?;
        fs::create_dir_all(path.join("examples"))?;

        self.create_readme(name, description, "common", path)?;
        Ok(())
    }

    fn create_readme(
        &self,
        name: &str,
        description: &str,
        project_type: &str,
        path: &Path,
    ) -> Result<()> {
        let mut content = format!(
            r#"# {name}

{description}

## Overview

This is a {project_type} project created with the project initialization tool.

## Setup

"#
        );

        match project_type {
            "python" => {
                content.push_str(
                    r#"
1. Create and activate a virtual environment:
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   ```
2. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
"#,
                );
            }
            "rust" => {
                content.push_str(
                    r#"
1. Build the project:
   ```bash
   cargo build
   ```
2. Run tests:
   ```bash
   cargo test
   ```
"#,
                );
            }
            _ => {}
        }

        fs::write(path.join("README.md"), content)?;
        Ok(())
    }
}

#[async_trait]
impl Agent for ProjectAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        let mut response = Message::new(format!("Project init received: {}", message.content));
        if let Some(metadata) = message.metadata {
            let state = self.current_state.clone().unwrap_or_else(|| "initial".to_string());
            let metadata = MessageMetadata::new("project_init".to_string())
                .with_personality(vec!["helpful".to_string(), "technical".to_string()])
                .with_state(state);
            response.metadata = Some(metadata);
        }
        Ok(response)
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.tools.execute(tool, params).await
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(self.current_state.clone().map(|s| State {
            name: s,
            data: None,
            prompt: None,
            transitions: None,
            validation: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_project_init() -> Result<()> {
        let config = AgentConfig {
            name: "project-init".to_string(),
            public_description: "Test project init".to_string(),
            instructions: "Test project initialization".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let agent = ProjectAgent::new(config).await?;
        let response = agent.process_message(Message::new("test".to_string())).await?;
        assert!(response.content.contains("Project init received"));
        Ok(())
    }
}
