use std::path::{Path, PathBuf};
use std::collections::HashMap;
use lazy_static::lazy_static;

/// Represents a project in the system
#[derive(Debug, Clone)]
pub struct Project {
    /// The name of the project
    pub name: String,
    /// The filesystem path to the project
    pub path: PathBuf,
    /// A short description of the project
    pub description: String,
    /// The parent project, if any
    pub parent: Option<String>,
}

/// Initialize the global projects list
///
/// This is exported for use by other modules that need to access
/// the full list of projects
pub fn get_projects() -> &'static HashMap<String, Project> {
    lazy_static! {
        static ref PROJECTS: HashMap<String, Project> = {
            let mut projects = HashMap::new();

            // Core projects
            projects.insert("New Project".to_string(), Project {
                name: "New Project".to_string(),
                path: PathBuf::from("~"),
                description: "Default project for new tasks".to_string(),
                parent: None,
            });

            projects.insert("regressiontestkit".to_string(), Project {
                name: "regressiontestkit".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit"),
                description: "Framework for regression testing and hardware integration".to_string(),
                parent: None,
            });

            projects.insert("madness_interactive".to_string(), Project {
                name: "madness_interactive".to_string(),
                path: PathBuf::from("~/lab/madness_interactive"),
                description: "Parent project for personal productivity tools and experiments".to_string(),
                parent: None,
            });

            projects.insert("Swarmonomicon".to_string(), Project {
                name: "Swarmonomicon".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/common/Swarmonomicon"),
                description: "Swarm-based agent system for task automation".to_string(),
                parent: Some("madness_interactive".to_string()),
            });

            projects.insert("Omnispindle".to_string(), Project {
                name: "Omnispindle".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/Common/Omnispindle"),
                description: "Python automation tool for task management".to_string(),
                parent: Some("madness_interactive".to_string()),
            });

            projects.insert("lab".to_string(), Project {
                name: "lab".to_string(),
                path: PathBuf::from("~/lab"),
                description: "Root directory for experiments and projects".to_string(),
                parent: None,
            });

            projects.insert(".hammerspoon".to_string(), Project {
                name: ".hammerspoon".to_string(),
                path: PathBuf::from("~/.hammerspoon"),
                description: "Hammerspoon automation scripts for macOS".to_string(),
                parent: None,
            });

            // RegressionTestKit ecosystem
            projects.insert("OculusTestKit".to_string(), Project {
                name: "OculusTestKit".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/OculusTestKit"),
                description: "Testing tools for Oculus devices".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("phoenix".to_string(), Project {
                name: "phoenix".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/phoenix"),
                description: "Regression test dashboard and control system".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("rust_ingest".to_string(), Project {
                name: "rust_ingest".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/rust_ingest"),
                description: "Rust-based data ingestion for regression testing".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("rtk-docs-host".to_string(), Project {
                name: "rtk-docs-host".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/rtk-docs-host"),
                description: "Documentation hosting for RegressionTestKit".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("gateway_metrics".to_string(), Project {
                name: "gateway_metrics".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/gateway_metrics"),
                description: "Metrics collection for gateways".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("http-dump-server".to_string(), Project {
                name: "http-dump-server".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/http-dump-server"),
                description: "Server for capturing and analyzing HTTP requests".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("teltonika_wrapper".to_string(), Project {
                name: "teltonika_wrapper".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/teltonika_wrapper"),
                description: "Wrapper for Teltonika device integration".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("ohmura-firmware".to_string(), Project {
                name: "ohmura-firmware".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/ohmura-firmware"),
                description: "Firmware for Ohmura devices".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            projects.insert("saws".to_string(), Project {
                name: "saws".to_string(),
                path: PathBuf::from("~/lab/regressiontestkit/saws"),
                description: "AWS utilities for regression testing".to_string(),
                parent: Some("regressiontestkit".to_string()),
            });

            // Other major projects
            projects.insert("Cogwyrm".to_string(), Project {
                name: "Cogwyrm".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/mobile/Cogwyrm"),
                description: "Mobile application for cognitive assistance".to_string(),
                parent: Some("madness_interactive".to_string()),
            });

            // Rust projects
            projects.insert("Tinker".to_string(), Project {
                name: "Tinker".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/rust/Tinker"),
                description: "Rust-based tinkering and experimental project".to_string(),
                parent: Some("rust-projects".to_string()),
            });

            projects.insert("EventGhost-Rust".to_string(), Project {
                name: "EventGhost-Rust".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/rust/EventGhost-Rust"),
                description: "Rust implementation of EventGhost".to_string(),
                parent: Some("rust-projects".to_string()),
            });

            // Python projects
            projects.insert("mcp-personal-jira".to_string(), Project {
                name: "mcp-personal-jira".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/python/mcp-personal-jira"),
                description: "Personal Jira integration tool".to_string(),
                parent: Some("python-projects".to_string()),
            });

            projects.insert("mqtt-get-var".to_string(), Project {
                name: "mqtt-get-var".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/python/mqtt-get-var"),
                description: "MQTT variable getter utility".to_string(),
                parent: Some("python-projects".to_string()),
            });

            projects.insert("dvtTestKit".to_string(), Project {
                name: "dvtTestKit".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/python/dvtTestKit"),
                description: "Testing tools for device validation".to_string(),
                parent: Some("python-projects".to_string()),
            });

            projects.insert("EventGhost-py".to_string(), Project {
                name: "EventGhost-py".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/python/EventGhost"),
                description: "Python implementation of EventGhost".to_string(),
                parent: Some("python-projects".to_string()),
            });

            // Project root directories
            projects.insert("projects-root".to_string(), Project {
                name: "projects-root".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects"),
                description: "Root directory for all projects".to_string(),
                parent: Some("madness_interactive".to_string()),
            });

            projects.insert("common-projects".to_string(), Project {
                name: "common-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/common"),
                description: "Common projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("mobile-projects".to_string(), Project {
                name: "mobile-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/mobile"),
                description: "Mobile projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("python-projects".to_string(), Project {
                name: "python-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/python"),
                description: "Python projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("lua-projects".to_string(), Project {
                name: "lua-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/lua"),
                description: "Lua projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("powershell-projects".to_string(), Project {
                name: "powershell-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/powershell"),
                description: "PowerShell projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("rust-projects".to_string(), Project {
                name: "rust-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/rust"),
                description: "Rust projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            projects.insert("tasker-projects".to_string(), Project {
                name: "tasker-projects".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker"),
                description: "Tasker projects directory".to_string(),
                parent: Some("projects-root".to_string()),
            });

            // Lua projects
            projects.insert("hammerspoon-proj".to_string(), Project {
                name: "hammerspoon-proj".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/lua/hammerspoon"),
                description: "Hammerspoon-related projects".to_string(),
                parent: Some("lua-projects".to_string()),
            });

            // PowerShell projects
            projects.insert("WinSystemSnapshot".to_string(), Project {
                name: "WinSystemSnapshot".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/powershell/WinSystemSnapshot"),
                description: "Windows system snapshot tool".to_string(),
                parent: Some("powershell-projects".to_string()),
            });

            // OS projects
            projects.insert("DisplayPhotoTime".to_string(), Project {
                name: "DisplayPhotoTime".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/OS/windows/DisplayPhotoTime"),
                description: "Photo display timing tool for Windows".to_string(),
                parent: Some("madness_interactive".to_string()),
            });

            // Tasker projects
            projects.insert("Verbatex".to_string(), Project {
                name: "Verbatex".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/Verbatex"),
                description: "Tasker project for text manipulation".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("RunedManifold".to_string(), Project {
                name: "RunedManifold".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/RunedManifold"),
                description: "Tasker project for symbolic processing".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("PhilosophersAmpoule".to_string(), Project {
                name: "PhilosophersAmpoule".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/PhilosophersAmpoule"),
                description: "Tasker project for philosophical inquiries".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("Ludomancery".to_string(), Project {
                name: "Ludomancery".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/Ludomancery"),
                description: "Tasker project for game-related automation".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("Fragmentarium".to_string(), Project {
                name: "Fragmentarium".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/Fragmentarium"),
                description: "Tasker project for fragment management".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("EntropyVector".to_string(), Project {
                name: "EntropyVector".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/EntropyVector"),
                description: "Tasker project for entropy manipulation".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("ContextOfficium".to_string(), Project {
                name: "ContextOfficium".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/ContextOfficium"),
                description: "Tasker project for context-aware task management".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects.insert("AnathemaHexVault".to_string(), Project {
                name: "AnathemaHexVault".to_string(),
                path: PathBuf::from("~/lab/madness_interactive/projects/tasker/AnathemaHexVault"),
                description: "Tasker project for encrypted storage".to_string(),
                parent: Some("tasker-projects".to_string()),
            });

            projects
        };
    }

    &PROJECTS
}

/// Get a flattened list of project names with descriptions
///
/// Useful for AI models that need to determine which project a task belongs to
pub fn get_project_descriptions() -> Vec<(String, String)> {
    get_projects()
        .iter()
        .map(|(name, project)| (name.clone(), project.description.clone()))
        .collect()
}

/// Get a project by name
pub fn get_project(name: &str) -> Option<&'static Project> {
    get_projects().get(name)
}

/// Get the default project name
pub fn get_default_project() -> &'static str {
    "madness_interactive"
}

/// Get a brief description of a project by name
pub fn get_project_description(project_name: &str) -> Option<&'static str> {
    match project_name {
        "madness_interactive" => Some("Parent project for personal productivity tools and experiments"),
        "Swarmonomicon" => Some("Swarm-based agent system for task automation"),
        "Omnispindle" => Some("Python automation tool for task management"),
        "regressiontestkit" => Some("Framework for regression testing and hardware integration"),
        ".hammerspoon" => Some("Hammerspoon automation scripts for macOS"),
        "mqtt-get-var" => Some("MQTT variable getter utility"),
        "EventGhost-Rust" => Some("Rust implementation of EventGhost"),
        _ => None,
    }
}

/// Get a formatted list of projects and descriptions for use in AI prompts
pub fn get_project_descriptions_text() -> String {
    r#"- madness_interactive: Parent project for personal productivity tools and experiments
- Swarmonomicon: Swarm-based agent system for task automation
- Omnispindle: Python automation tool for task management
- regressiontestkit: Framework for regression testing and hardware integration
- .hammerspoon: Hammerspoon automation scripts for macOS
- mqtt-get-var: MQTT variable getter utility
- EventGhost-Rust: Rust implementation of EventGhost
- rust_ingest: Rust-based data ingestion for regression testing
- phoenix: Regression test dashboard and control system
- Tinker: Rust-based tinkering and experimental project"#.to_string()
}
