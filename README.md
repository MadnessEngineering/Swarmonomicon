# Swarmonomicon: Rust Realtime Agents

## Inspiration and Credits

This project is a Rust reimplementation inspired by the [OpenAI Realtime Agents Demo](https://github.com/openai/openai-realtime-agents). The original project, created by Noah MacCallum and Ilan Bigio, demonstrates advanced agentic patterns built on top of a Realtime API.
My version is designed with plans to later become a [Tinker](https://github.com/DanEdens/Tinker) module.

### Original Project Highlights
The original OpenAI Realtime Agents project showcases:
- Sequential agent handoffs
- Background escalation to intelligent models
- State machine-driven user interactions
- Realtime voice application prototyping

## Our Implementation

Our Rust implementation aims to explore similar concepts of multi-agent systems, focusing on:
- Websocket-based realtime communication
- Modular agent system with configurable tools and behaviors
- Async runtime using tokio
-

### Current Implementation Status

#### Completed ✅
- Basic agent system with registry and transfer capabilities
- Websocket communication layer
- REST API endpoints for agent management
- Greeter and Haiku example agents
- Tool system with support for custom executors
- Configuration management for agents and tools

#### In Progress 🚧
- Session management with turn detection
- Audio handling and voice activity detection
- Background LLM calls for high-stakes decisions
- State machine validation for collecting user information
- Frontend UI components

## Setup

- This is a Rust project using tokio for async runtime
- Install dependencies with `cargo build`
- Run tests with `cargo test`
- Start the server with `cargo run`
- The server will start on [http://localhost:3000](http://localhost:3000)

## Architecture

### Core Components

1. **Agent System**
   - `AgentRegistry`: Manages available agents
   - `TransferService`: Handles agent transfers and message routing
   - `Agent` trait: Interface for implementing custom agents

2. **API Layer**
   - REST endpoints for agent management
   - Websocket handler for realtime communication
   - Session management (in progress)

3. **Tool System**
   - `ToolExecutor` trait for implementing custom tools
   - Support for async tool execution
   - Mqtt topic structure for agent state exchange

### Configuration

Agent configurations are defined in code, with support for:
- Custom instructions
- Tool assignments
- Downstream agent connections
- State machine definitions (in progress)

## Contributing

Contributions are welcome! Open Issues, I welcome them.

## Original Project Contributors
- Noah MacCallum - [noahmacca](https://x.com/noahmacca)
- Ilan Bigio - [ibigio](https://github.com/ibigio)

## License
MIT
