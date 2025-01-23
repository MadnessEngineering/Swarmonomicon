# Realtime API Agents Demo

This is a Rust implementation of more advanced, agentic patterns built on top of the Realtime API. In particular, this demonstrates:
- Sequential agent handoffs according to a defined agent graph (taking inspiration from [OpenAI Swarm](https://github.com/openai/swarm))
- Websocket-based realtime communication
- Modular agent system with configurable tools and behaviors

## Current Implementation Status

### Completed ✅
- Basic agent system with registry and transfer capabilities
- Websocket communication layer
- REST API endpoints for agent management
- Greeter and Haiku example agents
- Tool system with support for custom executors
- Configuration management for agents and tools

### In Progress 🚧
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
   - Built-in tools for agent transfer

### Configuration

Agent configurations are defined in code, with support for:
- Custom instructions
- Tool assignments
- Downstream agent connections
- State machine definitions (in progress)

## Contributing

See the [Rust Conversion TODO](https://github.com/openai/swarm/blob/main/CONTRIBUTING.md) section for areas that need work.

## Core Contributors
- Noah MacCallum - [noahmacca](https://x.com/noahmacca)
- Ilan Bigio - [ibigio](https://github.com/ibigio)

## License
MIT
