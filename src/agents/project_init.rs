use std::fs;
use std::path::Path;
use std::process::Command;
use crate::types::{Agent, AgentConfig, AgentResponse};
use crate::Result;

pub struct ProjectInitAgent {
    config: AgentConfig,
}

impl ProjectInitAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
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

    async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        // Parse command: create <type> <name> <description>
        let parts: Vec<&str> = message.split_whitespace().collect();
        
        if parts.len() < 4 || parts[0] != "create" {
            return Ok(AgentResponse {
                content: "Usage: create <type> <name> <description>".to_string(),
                should_transfer: false,
                transfer_to: None,
            });
        }

        let project_type = parts[1];
        let name = parts[2];
        let description = parts[3..].join(" ");

        // Validate project type
        if !["python", "rust", "common"].contains(&project_type) {
            return Ok(AgentResponse {
                content: "Project type must be one of: python, rust, common".to_string(),
                should_transfer: false,
                transfer_to: None,
            });
        }

        // Create project directory
        let base_dir = Path::new("projects").join(project_type);
        let project_dir = base_dir.join(name);

        if project_dir.exists() {
            return Ok(AgentResponse {
                content: format!("Error: Project directory {} already exists!", project_dir.display()),
                should_transfer: false,
                transfer_to: None,
            });
        }

        fs::create_dir_all(&project_dir)?;

        // Initialize project based on type
        match project_type {
            "python" => self.init_python_project(name, &description, &project_dir)?,
            "rust" => self.init_rust_project(name, &description, &project_dir)?,
            "common" => self.init_common_project(name, &description, &project_dir)?,
            _ => unreachable!(),
        }

        Ok(AgentResponse {
            content: format!(
                "Project {} created successfully in {}\nType: {}\nDescription: {}",
                name,
                project_dir.display(),
                project_type,
                description
            ),
            should_transfer: false,
            transfer_to: None,
        })
    }
} 
