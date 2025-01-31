# Swarmonomicon API Documentation

## Overview
This document provides comprehensive documentation for the Swarmonomicon API, including endpoints, request/response formats, and authentication methods.

## Core Concepts

### Agents
Agents are autonomous entities that can process messages, manage state, and interact with tools. Each agent has:
- A unique name
- A set of instructions
- Available tools
- Downstream agent connections
- Optional state machine configuration

### Todo Tasks
Tasks that can be delegated between agents with the following properties:
- `description`: Task description (unique)
- `status`: TodoStatus (Pending, InProgress, Completed, Failed)
- `assigned_agent`: Optional agent assigned to the task
- `context`: Optional context information
- `error`: Optional error message if task failed
- `created_at`: RFC3339 formatted timestamp
- `updated_at`: RFC3339 formatted timestamp

## Tools

### Todo Tool
The todo tool provides functionality for managing todo tasks stored in a MongoDB database.

#### Commands:

1. **Add Todo**
```json
{
  "command": "add",
  "description": "Task description",
  "context": "Optional context information"
}
```

2. **List Todos**
```json
{
  "command": "list"
}
```

3. **Complete Todo**
```json
{
  "command": "complete",
  "description": "Task description"
}
```

4. **Fail Todo**
```json
{
  "command": "fail", 
  "description": "Task description"
}
```

## API Endpoints

### Agent Management

#### GET /api/agents
Lists all available agents in the system.

**Response:**
```json
{
  "agents": [
    {
      "name": "string",
      "description": "string",
      "tools": [
        {
          "name": "string",
          "description": "string",
          "parameters": {
            "key": "string"
          }
        }
      ],
      "downstream_agents": ["string"]
    }
  ]
}
```

#### POST /api/agents/{agent_name}/message
Send a message to a specific agent.

**Request:**
```json
{
  "content": "string",
  "metadata": {
    "agent": "string",
    "state": "string",
    "personality_traits": ["string"],
    "transfer_target": "string",
    "context": {
      "key": "string"
    }
  }
}
```

**Response:**
```json
{
  "content": "string",
  "metadata": {
    "agent": "string",
    "state": "string",
    "personality_traits": ["string"],
    "transfer_target": "string",
    "context": {
      "key": "string"
    }
  }
}
```

### Todo Management

#### GET /api/todos
Retrieve all todo tasks.

**Response:**
```json
{
  "todos": [
    {
      "id": "string",
      "description": "string",
      "priority": "Low|Medium|High|Critical",
      "source_agent": "string",
      "target_agent": "string",
      "status": "Pending|InProgress|Completed|Failed",
      "created_at": 0,
      "completed_at": null
    }
  ]
}
```

#### POST /api/todos
Create a new todo task.

**Request:**
```json
{
  "description": "string",
  "priority": "Low|Medium|High|Critical",
  "source_agent": "string",
  "target_agent": "string"
}
```

## WebSocket API

### Connection
WebSocket endpoint: `ws://host/api/ws`

### Message Format
```json
{
  "content": "string",
  "metadata": {
    "agent": "string",
    "state": "string",
    "personality_traits": ["string"],
    "transfer_target": "string",
    "context": {
      "key": "string"
    }
  }
}
```

## Error Handling
All API endpoints may return the following error responses:

- 400 Bad Request: Invalid input parameters
- 401 Unauthorized: Authentication required
- 403 Forbidden: Insufficient permissions
- 404 Not Found: Resource not found
- 500 Internal Server Error: Server-side error

Error response format:
```json
{
  "error": {
    "code": "string",
    "message": "string",
    "details": {}
  }
}
```

## Rate Limiting
- Default rate limit: 100 requests per minute per IP
- WebSocket connections: Maximum 10 concurrent connections per IP

## Best Practices
1. Always check response status codes
2. Implement exponential backoff for retries
3. Keep WebSocket connections alive with periodic heartbeats
4. Handle errors gracefully
5. Validate input before sending to API 
