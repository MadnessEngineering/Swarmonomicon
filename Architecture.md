# Swarmonomicon Architecture

## Overview
Swarmonomicon is a multi-agent system that coordinates different specialized agents to handle various tasks like git operations, project initialization, and creative content generation. The system uses a transfer service to manage communication between agents and maintains a global registry of available agents.

## Core Components

### 1. Agent System
- **Base Agent Trait**: Defines core functionality all agents must implement
  - Message processing
  - Tool handling
  - State management
  - Configuration

### 2. Registry System
- **Global Registry**: Maintains references to all available agents
  - Thread-safe access via `Arc<RwLock<AgentRegistry>>`
  - Dynamic agent registration
  - Agent lookup by name

### 3. Transfer Service
- **State Machine**: Manages transitions between agents
- **Message Routing**: Directs messages to appropriate agents
- **Context Preservation**: Maintains context across agent transfers

### 4. Specialized Agents
1. **Git Assistant Agent**
   - Handles git operations
   - Generates commit messages
   - Manages branches and merges

2. **Project Init Agent**
   - Creates new project structures
   - Sets up configuration files
   - Initializes git repositories

3. **Haiku Agent**
   - Generates creative content
   - Integrates with git for committing haikus

4. **Greeter Agent**
   - Entry point for user interaction
   - Command routing
   - Help system

## Current Implementation Issues

### Agent Registration
1. Type Mismatch: The registry expects agents implementing the `Agent` trait, but we're wrapping them in multiple layers:
   ```rust
   Arc<RwLock<AgentImpl>> // Current structure
   ```

### Concurrency Model
1. Thread Safety: Using `Arc<RwLock>` for shared access
2. Async Operations: Using Tokio for async runtime
3. Need to ensure proper locking patterns

### Message Flow
1. Command Line Interface → Greeter → Specialized Agents
2. Inter-agent communication through Transfer Service
3. State preservation during transfers

## Design Principles
1. Thread-safe agent access
2. Async-first architecture
3. Modular agent system
4. Clear ownership boundaries
5. Type-safe message passing
