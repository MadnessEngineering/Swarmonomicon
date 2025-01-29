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

### 4. AI Communication Layer
- **Centralized AI Client**: Manages all LLM interactions
  - Configurable endpoint (default: local LM Studio)
  - Consistent message formatting
  - Conversation history management
  - System prompt handling
  - Model configuration

### 5. Specialized Agents
1. **Git Assistant Agent**
   - Handles git operations
   - Generates commit messages using AI
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
   - AI-powered conversation management

## Current Implementation Status

### Completed Features
1. Centralized AI client for consistent LLM interaction
2. Thread-safe agent registry with proper locking patterns
3. Async-first architecture with proper error handling
4. WebSocket-based real-time communication
5. Modular agent system with configurable tools

### In Progress
1. State machine improvements for complex workflows
2. Enhanced context preservation during transfers
3. Better error handling for AI communication
4. Improved conversation history management

## Design Principles
1. Thread-safe agent access
2. Async-first architecture
3. Modular agent system
4. Clear ownership boundaries
5. Type-safe message passing
6. Centralized AI communication
7. Consistent error handling
