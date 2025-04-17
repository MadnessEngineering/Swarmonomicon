# Swarmonomicon Architecture

## Overview
Swarmonomicon is a multi-agent system that coordinates different specialized agents to handle various tasks like git operations, project initialization, and creative content generation. The system uses a transfer service to manage communication between agents and maintains a global registry of available agents.

## Core Components

### 1. Agent System ✅
- **Base Agent Trait**: Defines core functionality all agents must implement
  - Message processing ✅
  - Tool handling ✅
  - State management ✅
  - Configuration ✅
  - Todo list integration ✅
- **Implementation Status**:
  - Base trait well-defined with async methods
  - Tool execution system in place
  - State machine implementation needs improvement
  - Configuration system works but needs better validation
  - Todo list integration with task processing complete
  - Concurrent task processing with rate limiting

### 2. Registry System ✅
- **Global Registry**: Maintains references to all available agents
  - Thread-safe access via `Arc<RwLock<AgentRegistry>>` ✅
  - Dynamic agent registration ✅
  - Agent lookup by name ✅
  - Feature-gated agent loading ✅
- **Implementation Status**:
  - Fully implemented with proper concurrency control
  - Good test coverage
  - Could benefit from better error handling on registration
  - Lazy static initialization for global registry

### 3. Transfer Service 🔄
- **State Machine**: Manages transitions between agents
  - Basic transitions working ✅
  - Complex workflows need improvement ⚠️
  - State validation incomplete ⚠️
  - State persistence needed ⚠️
- **Message Routing**: Directs messages to appropriate agents ✅
  - Proper locking for concurrent access ✅
  - Error handling for missing agents ✅
  - Message metadata preservation ✅
- **Context Preservation**: Maintains context across agent transfers ⚠️
  - Basic context passing works
  - Need better conversation history
  - State preservation needs improvement
  - Missing proper validation for circular transfers
- **Implementation Status**:
  - Basic transfer functionality works
  - State preservation needs improvement
  - Error handling could be more robust
  - Missing proper validation for circular transfers
  - Needs better conversation context management

### 4. AI Communication Layer ✅
- **Centralized AI Client**: Manages all LLM interactions
  - Configurable endpoint (default: local LM Studio) ✅
  - Consistent message formatting ✅
  - Conversation history management ✅
  - System prompt handling ✅
  - Model configuration ✅
  - Rate limiting and resource protection ✅
  - GPT-4 Batch Processing implemented ✅
- **Implementation Status**:
  - Well-implemented with proper abstraction
  - Good error handling
  - Rate limiting added
  - Proper resource management
  - GPT-4 batch processing tool with request pooling and token tracking
  - Could use better model fallback strategies
  - Needs better prompt management system

### 5. Task Processing System ✅
- **Todo List**: Manages tasks across agents
  - Task persistence ✅
  - Concurrent access handling ✅
  - Priority-based processing ✅
  - Task status tracking ✅
  - AI-powered task enhancement ✅
- **Implementation Status**:
  - Fully implemented with task persistence
  - Concurrent processing with rate limiting
  - Good error handling
  - Dual description system (original + enhanced)
  - Needs better monitoring
  - Could use better metrics collection

### 6. Specialized Agents
1. **Git Assistant Agent** ✅
   - Handles git operations ✅
   - Generates commit messages using AI ✅
   - Manages branches and merges ✅
   - Implementation complete with good test coverage
   - Needs better error recovery

2. **Project Init Agent** ⚠️
   - Creates new project structures ✅
   - Sets up configuration files ⚠️
   - Initializes git repositories ✅
   - Needs better template management
   - Missing language-specific optimizations

3. **Haiku Agent** 🔄
   - Generates creative content ✅
   - Basic implementation complete ✅
   - Integration with git needs improvement ⚠️
   - State machine enhancements needed

4. **Greeter Agent** ✅
   - Entry point for user interaction ✅
   - Command routing ✅
   - Help system ✅
   - AI-powered conversation management ✅
   - Well implemented with good test coverage
   - Could use better personality traits

5. **Browser Agent** 🔄
   - Handles browser automation tasks ✅
   - Browser integration functional ✅
   - Needs better error handling ⚠️
   - Requires improved state management

6. **RL Agent** 🔄
   - Reinforcement Learning capabilities ✅
   - Basic implementation complete with Flappy Bird environment ✅
   - Q-Learning implementation ✅
   - Needs improved training infrastructure

### 7. Tool System ✅
- **General Tools**:
  - Git operations ✅
  - Todo management ✅
  - Project setup ✅
  - GPT-4 batch processing ✅
  - YOLO object detection ✅
  - Goose performance testing ✅
  - Screenshot detection ✅

- **Implementation Status**:
  - Well-defined tool execution system
  - Good abstraction for tool definition
  - Support for async tool execution
  - Rate limiting for external service calls
  - Good error handling and reporting

## Current Implementation Status

### Completed Features ✅
1. Centralized AI client for consistent LLM interaction
2. Thread-safe agent registry with proper locking patterns
3. Async-first architecture with proper error handling
4. WebSocket-based real-time communication
5. Modular agent system with configurable tools
6. Concurrent task processing with rate limiting
7. Resource management and cleanup
8. Task processing system with prioritization
9. Feature-gated agent loading system
10. Basic state machine implementation
11. GPT-4 batch processing with efficient handling
12. Configurable AI endpoint and model selection
13. Object detection and screenshot detection tools
14. Basic Reinforcement Learning implementation

### In Progress 🔄
1. State machine improvements for complex workflows
2. Enhanced context preservation during transfers
3. Better error handling for AI communication
4. Improved conversation history management
5. Task system monitoring and metrics
6. Agent-specific tool support
7. Test coverage improvements
8. Prompt management system
9. Agent personality traits
10. Language-specific project templates
11. Browser agent enhancements
12. RL training infrastructure improvements

### Pending ⚠️
1. Fully robust HaikuAgent implementation
2. Task processing dashboard
3. System health monitoring
4. Performance benchmarking
5. API documentation improvements
6. Integration test suite expansion
7. Circular transfer validation
8. Model fallback strategies
9. Template management system
10. Metrics collection and analysis
11. MQTT logging for log agent

## Module Organization

The project is organized into several key modules:

### 1. Agents (`src/agents/`)
- Different agent implementations (greeter, git_assistant, haiku, browser, etc.)
- Agent wrapper for type handling
- Transfer service for inter-agent communication

### 2. Types (`src/types/`)
- Core type definitions (Agent, Message, Tool, etc.)
- Todo system types (TodoList, TodoTask, etc.)
- Project management types

### 3. Tools (`src/tools/`)
- Tool implementations (git, todo, project, yolo, etc.)
- GPT batch processing utility
- Object detection tools

### 4. API (`src/api/`)
- REST endpoints
- WebSocket handler
- Route definitions

### 5. Config (`src/config/`)
- Configuration management
- Environment variable handling

### 6. AI (`src/ai/`)
- AI client implementation
- Language model integration
- Conversation management

### 7. State (`src/state/`)
- State machine definitions
- State transition logic

## Design Principles
1. Thread-safe agent access ✅
2. Async-first architecture ✅
3. Modular agent system ✅
4. Clear ownership boundaries ✅
5. Type-safe message passing ✅
6. Centralized AI communication ✅
7. Consistent error handling 🔄
8. Resource management with RAII ✅
9. Rate limiting and protection ✅
10. Structured logging and monitoring ⚠️

## Implementation Details

### Error Handling
- Using `anyhow` and `thiserror` for error propagation
- Custom error types for specific domains
- Proper error conversion between types
- Needs better error recovery strategies

### Concurrency
- Using `tokio` for async runtime
- `Arc<RwLock<T>>` for shared state
- Semaphores for rate limiting
- RAII-based resource management

### Testing
- Unit tests for core components
- Integration tests for agent system
- Mock implementations for AI and tools
- Test isolation
- Several tests failing, needs attention

### Monitoring
- Basic tracing implementation
- Structured logging
- Needs metrics collection
- Needs system health monitoring
- Missing performance benchmarks

### Security
- API key management
- Rate limiting
- Resource protection
- Needs better access control
- Missing audit logging
