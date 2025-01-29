# Swarmonomicon: The Mad Tinker's Codex

🛠️ Welcome to the Swarmonomicon,\
A fusion of meticulous craftsmanship and heavily version controlled chaos.\
Inspired by OpenAI's Realtime Agents Demo, this project doesn't just reimagine agent collaboration—\
It invites you to dive headfirst into the Uncharted 🌀 and the Unhinged. 🤖🦾

Here, Rust-powered agents weave intricate patterns of logic, improvisation, and mischief.\
It's a spellbook for Mad Tinkerers ready to push boundaries and embrace the unpredictable.\
Throw UX in the trash, grab an extra keyboard, a bin of raspiberries and Buckle up.

⚙️ Tinker responsibly. \
Some assembly required[^1].\
Unintended Hilarity guaranteed[^2].


[^1]: Always assume "some" means "extensive" when dealing with Tinkers.

[^2]: May cause mqtt related restructuring of your entire codebase[^3].

[^3]: You're welcome, this is a feature not a bug.

## Inspiration and Credits

This project is a Rust reimplementation inspired by the [OpenAI Realtime Agents Demo](https://github.com/openai/openai-realtime-agents). The original project, created by [Noah MacCallum](https://x.com/noahmacca)] and [Ilan Bigio](https://github.com/ibigio), demonstrates advanced agentic patterns built on top of a Realtime API.
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


### Current Implementation Status

#### Completed ✅
- Basic agent system with registry and transfer capabilities
- Websocket communication layer
- REST API endpoints for agent management
- Greeter and Haiku example agents
- Tool system with support for custom executors
- Configuration management for agents and tools
- Centralized AI client with LM Studio integration
- Intelligent conversation handling with history
- Git operations with AI-powered commit messages

#### In Progress 🚧
- Adding entry point for greeter
- Update todo_worker to keep a record of tasks and to log more items
- Add mqtt logging for log agent to watch and react to
- Enhance conversation context preservation
- Improve error handling for AI communication
- Add more sophisticated state machine transitions

## Setup

### Prerequisites
- Rust toolchain (latest stable)
- LM Studio running locally (default: http://127.0.0.1:1234)
- Git (for version control features)

### Installation
1. Clone the repository
2. Install dependencies with `cargo build`
3. Start LM Studio with the Qwen model
4. Run tests with `cargo test`
5. Start the server with `cargo run`
6. The server will start on [http://localhost:3000](http://localhost:3000)

### Configuration
The system can be configured through environment variables:
- `AI_ENDPOINT`: LLM API endpoint (default: http://127.0.0.1:1234)
- `AI_MODEL`: Model to use (default: qwen2.5-7b-instruct)
- `RUST_LOG`: Logging level (default: info)

## Architecture

### Core Components

1. **Agent System**
   - `AgentRegistry`: Manages available agents
   - `TransferService`: Handles agent transfers and message routing
   - `Agent` trait: Interface for implementing custom agents
   - `AiClient`: Centralized LLM communication

2. **API Layer**
   - REST endpoints for agent management
   - Websocket handler for realtime communication
   - Session management
   - AI-powered conversation handling

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

## License
MIT

## Subrepo Structure

Welcome, the few and the Mad, to the wondrous world of subrepos!\
This project is but a cog in the grand machine of the [Madness Interactive](https://github.com/DanEdens/madness_interactive) repository—\
A playground for my various Mad Science and other monstrosities of Automation.\
Embrace the mess of modular development, each project is but a part of the glorious, interconnected ***machine***.

Ferrum Corde!

## Features

- Agent-based architecture for modular task handling
- Tool registry for extensible functionality
- State machine support for complex workflows
- WebSocket API for real-time communication
- CLI interface for common operations

### Git Operations

The framework includes a Git assistant agent that can be used via CLI:

```bash
# Auto-generate commit message
swarm git

# Commit with specific message
swarm git -m "feat: add new feature"

# Create and switch to new branch
swarm git -b feature/new-branch

# Merge current branch into target
swarm git -t main
```

The Git assistant uses AI to generate meaningful commit messages based on the changes in your working directory.
