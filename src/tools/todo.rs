use std::collections::HashMap;
use async_trait::async_trait;
use chrono::Utc;
use crate::tools::ToolExecutor;
use crate::agents::user_agent::{TodoItem, TodoStatus};
use crate::Result;
use std::fs;
use serde_json::{json, Value};

pub struct TodoTool;

impl TodoTool {
    pub fn new() -> Self {
        Self
    }

    fn read_todo_state(&self) -> Result<Value> {
        let contents = fs::read_to_string("todo_state.json")?;
        let state: Value = serde_json::from_str(&contents)?;
        Ok(state)
    }

    fn write_todo_state(&self, state: &Value) -> Result<()> {
        let contents = serde_json::to_string_pretty(state)?;
        fs::write("todo_state.json", contents)?;
        Ok(())
    }

    fn add_todo(&self, description: &str, context: Option<&str>) -> Result<String> {
        let mut state = self.read_todo_state()?;
        let now = Utc::now().to_rfc3339();

        let new_todo = json!({
            "description": description,
            "status": "Pending",
            "assigned_agent": null,
            "context": context,
            "error": null,
            "created_at": now,
            "updated_at": now
        });

        state["todos"].as_array_mut()
            .ok_or("Invalid todo state format")?
            .push(new_todo);

        self.write_todo_state(&state)?;
        Ok(format!("Added new todo: {}", description))
    }

    fn list_todos(&self) -> Result<String> {
        let state = self.read_todo_state()?;
        let todos = state["todos"].as_array()
            .ok_or("Invalid todo state format")?;

        if todos.is_empty() {
            return Ok("No todos found.".to_string());
        }

        let mut output = String::from("Current todos:\n");
        for (i, todo) in todos.iter().enumerate() {
            output.push_str(&format!("{}. {} ({})\n",
                i + 1,
                todo["description"].as_str().unwrap_or("Invalid description"),
                todo["status"].as_str().unwrap_or("Unknown status")
            ));
        }

        Ok(output)
    }

    fn update_todo_status(&self, index: usize, status: TodoStatus) -> Result<String> {
        let mut state = self.read_todo_state()?;
        let todos = state["todos"].as_array_mut()
            .ok_or("Invalid todo state format")?;

        if index >= todos.len() {
            return Err("Todo index out of range".into());
        }

        let now = Utc::now().to_rfc3339();
        todos[index]["status"] = json!(format!("{:?}", status));
        todos[index]["updated_at"] = json!(now);

        self.write_todo_state(&state)?;
        Ok(format!("Updated todo status to {:?}", status))
    }
}

#[async_trait]
impl ToolExecutor for TodoTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let command = params.get("command").ok_or("Missing command parameter")?;

        match command.as_str() {
            "add" => {
                let description = params.get("description").ok_or("Missing todo description")?;
                let context = params.get("context").map(|s| s.as_str());
                self.add_todo(description, context)
            }
            "list" => {
                self.list_todos()
            }
            "complete" => {
                let index = params.get("index")
                    .ok_or("Missing todo index")?
                    .parse::<usize>()
                    .map_err(|_| "Invalid todo index")?;
                self.update_todo_status(index, TodoStatus::Completed)
            }
            "fail" => {
                let index = params.get("index")
                    .ok_or("Missing todo index")?
                    .parse::<usize>()
                    .map_err(|_| "Invalid todo index")?;
                self.update_todo_status(index, TodoStatus::Failed)
            }
            _ => Err("Unknown todo command".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::fs;

    #[tokio::test]
    async fn test_todo_operations() -> Result<()> {
        // Create a temporary todo state file
        let temp_file = NamedTempFile::new()?;
        let initial_state = json!({
            "todos": [],
            "last_processed": null
        });
        fs::write(temp_file.path(), serde_json::to_string_pretty(&initial_state)?)?;

        let tool = TodoTool::new();

        // Test adding a todo
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Test todo".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Added new todo"));

        // Test listing todos
        let mut params = HashMap::new();
        params.insert("command".to_string(), "list".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Test todo"));

        // Cleanup: Explicitly drop the temp_file to ensure it's deleted
        drop(temp_file);

        Ok(())
    }
}
