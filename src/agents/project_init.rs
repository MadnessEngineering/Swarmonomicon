use std::fs;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, Tool, ToolCall, State};
use crate::tools::ToolRegistry;
use crate::Result;

pub struct ProjectInitAgent {
    config: AgentConfig,
    tools: ToolRegistry,
    current_state: Option<String>,
}

impl ProjectInitAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tools: ToolRegistry::create_default_tools(),
            current_state: None,
        }
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

#[async_trait::async_trait]
impl Agent for ProjectInitAgent {
    fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process_message(&mut self, message: &str) -> Result<Message> {
        // Parse command: create <type> <name> <description>
        let parts: Vec<&str> = message.split_whitespace().collect();
        
        if parts.len() < 4 || parts[0] != "create" {
            return Ok(Message {
                content: "Usage: create <type> <name> <description>".to_string(),
                role: "assistant".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                metadata: None,
            });
        }

        let project_type = parts[1];
        let name = parts[2];
        let description = parts[3..].join(" ");

        // Validate project type
        if !["python", "rust", "common"].contains(&project_type) {
            return Ok(Message {
                content: "Project type must be one of: python, rust, common".to_string(),
                role: "assistant".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                metadata: None,
            });
        }

        // Create project using the project tool
        let mut params = HashMap::new();
        params.insert("type".to_string(), project_type.to_string());
        params.insert("name".to_string(), name.to_string());
        params.insert("description".to_string(), description.clone());

        let tool_call = ToolCall {
            tool: "project".to_string(),
            parameters: params.clone(),
            result: None,
        };

        let result = self.tools.execute(&Tool {
            name: "project".to_string(),
            description: "Project initialization tool".to_string(),
            parameters: HashMap::new(),
        }, params).await?;

        Ok(Message {
            content: result,
            role: "assistant".to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            metadata: Some(MessageMetadata {
                tool_calls: Some(vec![tool_call]),
                state: self.current_state.clone(),
                confidence: None,
            }),
        })
    }

    async fn transfer_to(&mut self, _agent_name: &str) -> Result<()> {
        Ok(())
    }

    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        self.tools.execute(tool, params).await
    }

    fn get_current_state(&self) -> Option<&State> {
        None
    }
} 
