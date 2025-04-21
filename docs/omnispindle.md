# Omnispindle: Distributed Task Management System

*"The threads that weave reality itself!"*

## Overview

Omnispindle is a distributed task management system designed to orchestrate tasks across multiple agents and systems. It serves as the central nervous system for the Madness Interactive ecosystem, allowing for asynchronous task processing, resource management, and cross-project communication.

## Architecture

The Omnispindle system consists of several interconnected components:

### 1. MCP Todo Server (`mcp_todo_server.rs`)

The MCP Todo Server acts as the central hub for task management. It:

- Listens for task requests via MQTT
- Processes and validates incoming tasks
- Stores tasks in a persistent database
- Broadcasts task status updates
- Manages task prioritization
- Provides rate limiting to prevent system overload

### 2. Todo Worker (`todo_worker.rs`)

The Todo Worker processes tasks from the shared TodoList:

- Fetches pending tasks from the database
- Processes tasks based on priority and type
- Handles task retry logic for failed operations
- Provides real-time status updates via MQTT
- Manages concurrent task execution with rate limiting
- Routes tasks to appropriate processing agents

### 3. Shared TodoList System

A centralized storage system that:

- Maintains task state across system restarts
- Supports metadata attachment to tasks
- Provides task prioritization
- Handles concurrent access safely
- Supports filtering and querying

## Communication Flow

```
┌─────────────────┐     MQTT     ┌─────────────────┐
│                 │  (mcp/task)  │                 │
│  Client/Agent   ├─────────────►│  MCP Todo Server│
│                 │              │                 │
└─────────────────┘              └────────┬────────┘
                                          │
                                          │ TodoList Storage
                                          ▼
                                 ┌─────────────────┐
                                 │                 │
                                 │   Todo Worker   │
                                 │                 │
                                 └────────┬────────┘
                                          │
                                          │ Task Execution
                                          ▼
                                 ┌─────────────────┐
                                 │   Agent/Tool    │
                                 │   Execution     │
                                 └─────────────────┘
```

## Task Lifecycle

1. **Creation**: Tasks are created via MQTT messages to the `mcp/task` topic or directly through the TodoList API
2. **Validation**: The server validates and enhances the task with metadata
3. **Storage**: Tasks are stored in the TodoList with a unique ID and status
4. **Processing**: The Todo Worker fetches and processes tasks based on priority
5. **Execution**: Tasks are executed using the appropriate agent or tool
6. **Status Updates**: Status updates are published via MQTT to `task/status/{agent}`
7. **Completion/Failure**: Tasks are marked as completed or failed in the TodoList

## Configuration

### MCP Todo Server

Environment variables:
- `AWSIP`: MQTT broker address (default: localhost)
- `MQTTPORT`: MQTT broker port (default: 3003)

### Todo Worker

Environment variables:
- `AWSIP`: MQTT broker address (default: localhost)
- `MQTTPORT`: MQTT broker port (default: 3003)

## Task Format

### JSON Format

```json
{
  "description": "Task description",
  "priority": "high", // Options: low, medium, high, critical
  "context": "source context",
  "target_agent": "agent_name",
  "metadata": {
    "type": "feature|bugfix|documentation",
    "complexity": "low|medium|high",
    "estimated_time": "duration",
    "requires": "comma,separated,dependencies"
  }
}
```

### Simple Text Format

Tasks can also be submitted as plain text to the `todos/{agent}` topic, which will be processed with default settings.

## Error Handling

Omnispindle implements robust error handling:

- Retry logic for failed tasks (configurable attempts and backoff)
- Task status tracking for monitoring
- Detailed logging for debugging failures
- Rate limiting to prevent system overload

## Security Considerations

- MQTT communication should be secured in production environments
- Access control should be implemented for task submission
- Input validation is performed on all incoming tasks
- Rate limiting prevents denial of service attacks

## Future Enhancements

- Task scheduling with time-based execution
- Advanced task dependencies and workflows
- Enhanced monitoring and metrics
- Web-based administration interface
- Distributed worker scaling

## Integration with Other Systems

Omnispindle integrates with:

- Swarmonomicon agent system for task execution
- MQTT messaging system for communication
- MongoDB for persistent storage
- Local LLM systems for task enhancement

## API Reference

### TodoList API

Key methods:
- `create_task(description, priority, context, target_agent)`
- `get_next_task()`
- `mark_task_completed(id)`
- `mark_task_failed(id)`

### MQTT Topics

- `mcp/task/{agent}`: Submit tasks to specific agent
- `todos/{agent}`: Submit simple text tasks to specific agent
- `task/status/{agent}`: Receive task status updates

## Lessons Learned

1. Concurrent task processing requires careful rate limiting
2. State persistence is critical for system reliability
3. Robust error handling improves system stability
4. Task status updates provide valuable visibility
5. Standardized task formats simplify integration

---

*"The difference between order and chaos is measured only by the quality of your task management system!"* 