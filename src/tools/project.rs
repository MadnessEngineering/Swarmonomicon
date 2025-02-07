use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use async_trait::async_trait;
use crate::tools::ToolExecutor;
use anyhow::{Result, anyhow};

pub struct ProjectTool;

impl ProjectTool {
    pub fn new() -> Self {
        Self
    }

    fn init_python_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        // Create project structure
        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| anyhow!("Failed to create src directory: {}", e))?;
        fs::create_dir_all(src_dir.join(name)).map_err(|e| anyhow!("Failed to create project directory: {}", e))?;
        fs::create_dir_all(src_dir.join("tests")).map_err(|e| anyhow!("Failed to create tests directory: {}", e))?;

        // Create __init__.py files
        fs::write(src_dir.join(name).join("__init__.py"), "").map_err(|e| anyhow!("Failed to create __init__.py: {}", e))?;
        fs::write(src_dir.join("tests").join("__init__.py"), "").map_err(|e| anyhow!("Failed to create test __init__.py: {}", e))?;

        // Create requirements.txt
        fs::write(path.join("requirements.txt"), "# Core dependencies\n").map_err(|e| anyhow!("Failed to create requirements.txt: {}", e))?;

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
        fs::write(path.join("setup.py"), setup_content).map_err(|e| anyhow!("Failed to create setup.py: {}", e))?;

        self.create_readme(name, description, "python", path)?;
        Ok(())
    }

    fn init_rust_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        Command::new("cargo")
            .args(["init", "--name", name])
            .current_dir(path)
            .output()
            .map_err(|e| anyhow!("Failed to initialize Rust project: {}", e))?;

        self.create_readme(name, description, "rust", path)?;
        Ok(())
    }

    fn init_common_project(&self, name: &str, description: &str, path: &Path) -> Result<()> {
        fs::create_dir_all(path.join("src")).map_err(|e| anyhow!("Failed to create src directory: {}", e))?;
        fs::create_dir_all(path.join("docs")).map_err(|e| anyhow!("Failed to create docs directory: {}", e))?;
        fs::create_dir_all(path.join("examples")).map_err(|e| anyhow!("Failed to create examples directory: {}", e))?;

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

        fs::write(path.join("README.md"), content).map_err(|e| anyhow!("Failed to create README.md: {}", e))?;
        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for ProjectTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let project_type = params.get("type").ok_or_else(|| anyhow!("Missing project type"))?;
        let name = params.get("name").ok_or_else(|| anyhow!("Missing project name"))?;
        let description = params.get("description").ok_or_else(|| anyhow!("Missing project description"))?;

        // Validate project type
        if !["python", "rust", "common"].contains(&project_type.as_str()) {
            return Err(anyhow!("Project type must be one of: python, rust, common"));
        }

        // Create project directory
        let base_dir = Path::new("projects").join(project_type);
        let project_dir = base_dir.join(name);

        if project_dir.exists() {
            return Err(anyhow!("Project directory {} already exists!", project_dir.display()));
        }

        fs::create_dir_all(&project_dir).map_err(|e| anyhow!("Failed to create project directory: {}", e))?;

        // Initialize project based on type
        match project_type.as_str() {
            "python" => self.init_python_project(name, description, &project_dir)?,
            "rust" => self.init_rust_project(name, description, &project_dir)?,
            "common" => self.init_common_project(name, description, &project_dir)?,
            _ => unreachable!(),
        }

        Ok(format!(
            "Project {} created successfully in {}\nType: {}\nDescription: {}",
            name,
            project_dir.display(),
            project_type,
            description
        ))
    }
} 
