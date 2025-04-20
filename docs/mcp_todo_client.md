# MCP Todo Client Tool

The MCP Todo Client Tool provides a convenient interface for Swarmonomicon agents to interact with the MCP Todo Server. This allows agents to add, list, update, and complete todos, effectively collaborating with other tools and services in the system.

## Overview

The MCP Todo Client Tool uses MQTT to communicate with the MCP Todo Server, which manages a database of todos. This architecture enables asynchronous communication and event-driven workflows between different components of your system.

## Setup

1. Ensure you have access to an MQTT broker (such as Mosquitto) running on your local machine or a remote server.
2. Configure the MQTT connection parameters through environment variables:
   ```
   MQTT_HOST=localhost
   MQTT_PORT=1883
   ```

## Usage in Agents

Agents can use the MCP Todo Client Tool to:

- Create new todos with various priorities
- List existing todos with filtering
- Get details of specific todos
- Update todos (status, priority, etc.)
- Mark todos as completed
- Delete todos

## Command Reference

### Add a Todo

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "add".to_string());
params.insert("description".to_string(), "Task description".to_string());
params.insert("project".to_string(), "project-name".to_string());
params.insert("priority".to_string(), "medium".to_string()); // Optional: low, medium, high, critical
params.insert("target_agent".to_string(), "user".to_string()); // Optional: Which agent should handle this
```

### List Todos

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "list".to_string());
params.insert("filter".to_string(), r#"{"project": "example-project"}"#.to_string()); // Optional: JSON filter
params.insert("limit".to_string(), "10".to_string()); // Optional: Maximum number of todos to retrieve
```

### Get a Specific Todo

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "get".to_string());
params.insert("todo_id".to_string(), "<todo-id>".to_string());
```

### Update a Todo

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "update".to_string());
params.insert("todo_id".to_string(), "<todo-id>".to_string());
params.insert("updates".to_string(), r#"{"priority": "high", "status": "in-progress"}"#.to_string());
```

### Mark a Todo as Complete

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "complete".to_string());
params.insert("todo_id".to_string(), "<todo-id>".to_string());
```

### Delete a Todo

```rust
let mut params = HashMap::new();
params.insert("command".to_string(), "delete".to_string());
params.insert("todo_id".to_string(), "<todo-id>".to_string());
```

## Example

Here's a complete example of how to use the MCP Todo Client Tool in an agent:

```rust
async fn handle_task(tool_registry: &ToolRegistry) -> Result<()> {
    // Add a todo
    let mut params = HashMap::new();
    params.insert("command".to_string(), "add".to_string());
    params.insert("description".to_string(), "Research quantum physics".to_string());
    params.insert("project".to_string(), "science".to_string());
    
    let tool = Tool {
        name: "mcp_todo".to_string(),
        description: "MCP Todo Client Tool".to_string(),
        parameters: params,
    };
    
    let result = tool_registry.execute(&tool, tool.parameters.clone()).await?;
    println!("Added todo: {}", result);
    
    Ok(())
}
```

## Running the Example

You can run the included example to test the MCP Todo Client Tool:

```bash
cargo run --example mcp_todo_client
```

This will demonstrate adding, listing, getting, updating, and completing todos using the MCP Todo Client Tool.

## Testing

The included tests verify the functionality of the MCP Todo Client Tool by connecting to a real MCP Todo Server. To run the tests:

```bash
cargo test -- --ignored
```

Note that the tests are marked as `#[ignore]` by default since they require a running MCP Todo Server.

## Implementation Details

The MCP Todo Client Tool uses:

- MQTT for communication with the MCP Todo Server
- JSON for data serialization
- Asynchronous Rust with Tokio for concurrent operations
- Request/response pattern with timeouts and error handling 
