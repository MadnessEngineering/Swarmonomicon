# Swarmonomicon: The Mad Tinker's Codex

![Cover Art](docs/assets/Cover-Art.jpeg)

> *A spellbook for those who believe the correct number of agents is always one more than you currently have.*

Swarmonomicon is a **Rust-powered multi-agent orchestration system** with priority task queues, MQTT coordination, and enough emergent behavior to keep you guessing. Inspired by OpenAI's Realtime Agents Demo and rebuilt from the substrate up in async Rust — because if you're going to summon a swarm, you might as well do it without a garbage collector.

Each agent owns its own todo queue. Tasks cascade through priorities, get enhanced by local LLMs, classified into projects, and processed asynchronously while MQTT keeps the whole nervous system ticking. The machine learns. The machine *adapts*.[^1]

[^1]: "Adapts" is a generous word. "Fails gracefully and tries again with exponential backoff" is more accurate. Both are features.

---

## Where This Fits: The Madness Interactive Ecosystem

```
┌─────────────────────────────────────────────────────────────────┐
│                    madness_interactive                          │
│                                                                 │
│  ┌──────────────┐   MQTT (mcp/+)   ┌─────────────────────┐    │
│  │  Omnispindle │ ───────────────► │   Swarmonomicon     │    │
│  │  (MCP tools) │                  │  ┌───────────────┐  │    │
│  └──────────────┘                  │  │  Agent Swarm  │  │    │
│                                    │  │  Todo Queues  │  │    │
│  ┌──────────────┐   REST API       │  │  MQTT Intake  │  │    │
│  │  Inventorium │ ◄─────────────── │  └───────────────┘  │    │
│  │  (Dashboard) │                  │         │            │    │
│  └──────────────┘                  │         ▼            │    │
│                                    │      MongoDB         │    │
│                                    └─────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

- **Omnispindle** publishes tasks into Swarmonomicon via MQTT (`mcp/<agent>` topics)
- **Swarmonomicon** classifies, enhances, queues, and processes those tasks
- **Inventorium** reads the results via REST API to display in the dashboard
- **MongoDB** (`RTK_MONGO_URI` / `RTK_MONGO_DB`) is the shared persistence layer

Swarmonomicon is a planned module of [Tinker](https://github.com/DanEdens/Tinker). It currently lives as a git submodule at `projects/common/Swarmonomicon`.

---

## Architecture

### The Agent System

Each agent is an independent async entity implementing the `Agent` trait. Agents own their own todo queue, maintain state, and can delegate tasks to other agents. The `AgentRegistry` manages discovery; `TransferService` handles routing.

| Agent | Role |
|---|---|
| **Greeter** | Entry point. Routes incoming users and tasks to the right agent |
| **Git Assistant** | AI-powered commit messages, branch ops, merge helpers |
| **Haiku** | Creative generation demo. Also a useful smoke test |
| **Project Init** | Scaffolds new projects with sane defaults |
| **Browser** | Chromium automation (feature-flagged: `browser-agent`) |
| **RL Agent** | Q-learning framework, ships with a Flappy Bird environment |

Agents are enabled via Cargo feature flags — compile only what your deployment needs.

### The Task Queue System

Tasks flow through a MongoDB-backed queue with atomic priority scheduling. The `get_next_task()` call does a `findOneAndUpdate` sorted by `priority DESC, created_at ASC` — highest urgency, oldest first, claimed atomically to prevent double-processing.

**Priority levels** (ordered lowest → highest):

```rust
pub enum TaskPriority {
    Initial,   // just arrived, not yet evaluated
    Low,       // background, no rush
    Medium,    // regular work
    High,      // process soon
    Critical,  // drop everything
}
```

**Task lifecycle:**

```
Initial ──► Pending ──► Review ──► Completed
                   └──────────────► Failed
```

**`TodoTask` structure:**

```rust
pub struct TodoTask {
    pub id: String,                            // UUID
    pub description: String,                  // original, always preserved
    pub enhanced_description: Option<String>, // AI-improved version
    pub priority: TaskPriority,
    pub project: Option<String>,              // auto-classified if absent
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub status: TaskStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub due_date: Option<String>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
    pub ticket: Option<String>,
    pub last_modified: Option<i64>,
}
```

**AI Enhancement Layer:** When an AI client is available, `create_task_with_enhancement()` calls the local LLM to:
1. Rewrite the description with technical context
2. Predict a better priority (only upgraded, never downgraded)
3. Classify the task into a project (only set if not already provided)

The original description is always preserved. The enhanced version is additive.

### The MQTT Coordination Layer

MQTT is the central nervous system. The `mqtt_intake` binary subscribes to `mcp/+` and turns inbound messages into queued tasks. The topic path encodes the target agent — `mcp/git_assistant` routes to the Git assistant, `mcp/greeter` to the greeter.

**Full topic map:**

| Direction | Topic | Purpose |
|---|---|---|
| **Inbound** | `mcp/+` | Task creation — subtopic becomes `target_agent` |
| **Inbound** | `mcp_server/control` | `{"command": "status"}` or `{"command": "shutdown"}` |
| **Inbound** | `project/classify` | Project worker classification request |
| **Inbound** | `todo_worker/control` | Worker runtime control commands |
| **Outbound** | `response/{agent}/todo` | Task successfully created |
| **Outbound** | `response/{agent}/error` | Task creation failed |
| **Outbound** | `response/mcp_server/status` | Server status / shutdown confirmation |
| **Outbound** | `response/project/classify/{uuid}` | Per-request classification response |
| **Outbound** | `metrics/response/mqtt_intake` | Periodic `TaskMetrics` JSON (every 300s) |
| **Outbound** | `health/todo_worker` | Worker health status |

The `response/` prefix is intentional — it separates commands from responses and prevents the intake from processing its own output.[^2] All communications use **QoS 2 (ExactlyOnce)**.

[^2]: We learned this the hard way. The footnote from v0.1.0 that said "may cause mqtt related restructuring of your entire codebase" was autobiographical.

**Task creation flow via MQTT:**

```
Publish to mcp/greeter
      │
      ▼
mqtt_intake receives it
      │
      ├─► Subscribe to response/project/classify/{uuid}
      ├─► Publish to project/classify (classification request)
      ├─► Wait up to 30s for response
      │   └─► Timeout? Default to "madness_interactive"
      │
      ▼
TodoTool.execute() → MCP server → MongoDB insert
      │
      ├─► Publish response/greeter/todo (success)
      └─► Publish response/greeter/error (failure)
```

**Periodic metrics** (published to `metrics/response/mqtt_intake`):

```json
{
  "tasks_received": 127,
  "tasks_processed": 120,
  "tasks_failed": 5,
  "classification_success_rate": 97.6,
  "success_rate": 94.5,
  "uptime_seconds": 3600,
  "tasks_per_minute": 2.1
}
```

### Swarm Intelligence

Beyond individual agents, Swarmonomicon has a coordination layer for emergent multi-agent behavior:

**Consensus Protocol** — Agents vote on actions using configurable strategies:

| Strategy | Rule |
|---|---|
| `Majority` | >50% agreement required |
| `Plurality` | Most votes wins, always resolves |
| `Unanimous` | 100% agreement, or nothing |
| `Weighted` | Votes weighted by agent expertise score |

**Delegation** — Agents can hand off tasks mid-stream when a better-suited agent is available.

**Emergence** — Behavioral patterns that arise from agent interaction, not explicit programming.

**Shared Learning** — Knowledge propagates across the swarm. What one agent learns, the collective benefits from.

**Swarm Metrics** — Coordination performance monitoring.

### Task Intelligence

A subsystem of AI-powered analyzers that operate on tasks before and during processing:

| Component | Function |
|---|---|
| `SmartTodo` | AI-enhanced task processing with context injection |
| `Decomposer` | Breaks complex tasks into ordered subtasks |
| `PriorityPredictor` | ML-based priority estimation from task text |
| `DependencyLearner` | Learns which tasks tend to block which |
| `TimePredictor` | Estimates completion duration from history |
| `TaskHistory` | Stores outcomes to feed the predictors |

---

## Quick Start

### Prerequisites

- Rust (latest stable)
- MongoDB instance (set `RTK_MONGO_URI`)
- MQTT broker (Mosquitto or hosted — set `AWSIP` / `AWSPORT`)
- Optional: LM Studio or Ollama for AI enhancement

### Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `RTK_MONGO_URI` | *(required)* | MongoDB connection string |
| `RTK_MONGO_DB` | `swarmonomicon` | Database name |
| `AWSIP` | *(required for MQTT)* | MQTT broker hostname/IP |
| `AWSPORT` | *(required for MQTT)* | MQTT broker port |
| `AI_ENDPOINT` | `http://127.0.0.1:1234` | LLM API endpoint |
| `AI_MODEL` | `qwen2.5-7b-instruct` | Model name |
| `RUST_LOG` | `info` | Log level |

### Build & Run

```bash
# Build everything
cargo build

# Run the API server (port 3000)
cargo run --bin swarm

# Run the MQTT intake listener
cargo run --bin mqtt_intake

# Run the background todo worker
cargo run --bin todo_worker

# Run the MCP JSON-RPC server
cargo run --bin mcp_todo_server

# Train a Flappy Bird RL agent
cargo run --bin train_flappy --features rl

# Train with visualization
cargo run --bin train_flappy --features rl -- -v
```

### Docker (the lazy way)

```bash
# macOS/Linux
./docker-setup.sh

# Or directly:
docker compose up -d
```

Starts: API server (port 3000), MongoDB, Mosquitto MQTT broker. Configured to use a local Ollama instance at `host.docker.internal:11434`.

---

## API Reference

### Agent Management

```
GET  /api/agents              → list all agents
GET  /api/agents/:name        → agent details
POST /api/agents/:name/message → send a message to an agent
POST /api/agents/:name/send   → send a command to an agent
```

### Task Management

```
GET  /api/agents/:name/tasks          → list all tasks for agent
POST /api/agents/:name/tasks          → add task to agent's queue
GET  /api/agents/:name/tasks/:task_id → get specific task
```

### WebSocket

```
GET /ws → real-time bidirectional communication
```

**Add a task:**

```bash
curl -X POST http://localhost:3000/api/agents/greeter/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Welcome new user — onboard to the lab",
    "priority": "High",
    "source_agent": null
  }'
```

**Publish via MQTT:**

```bash
# Using mosquitto_pub
mosquitto_pub -h $AWSIP -p $AWSPORT -t mcp/greeter \
  -m '{"description": "Calibrate the flux capacitor", "priority": "Medium"}'

# Plain text also works
mosquitto_pub -h $AWSIP -p $AWSPORT -t mcp/git_assistant \
  -m "Generate commit message for current changes"
```

**Control the worker remotely:**

```bash
# Check status
mosquitto_pub -h $AWSIP -p $AWSPORT -t todo_worker/control \
  -m '{"command": "status"}'

# Graceful shutdown
mosquitto_pub -h $AWSIP -p $AWSPORT -t mcp_server/control \
  -m '{"command": "shutdown"}'
```

---

## CLI (Git Assistant)

```bash
# Auto-generate commit message from staged changes
swarm git

# Commit with specific message
swarm git -m "feat: add new feature"

# Create and switch to new branch
swarm git -b feature/new-branch

# Merge current branch into target
swarm git -t main
```

---

## Feature Flags

| Flag | Enables |
|---|---|
| `greeter-agent` | Greeter entry-point agent |
| `git-agent` | Git operations assistant |
| `haiku-agent` | Haiku generation agent |
| `project-init-agent` | Project scaffolding agent |
| `browser-agent` | Chromium browser automation |
| `rl` | Reinforcement learning framework + Flappy Bird |

Build only what you need:

```bash
cargo build --features "git-agent,greeter-agent"
```

---

## Worker Binaries

| Binary | Purpose |
|---|---|
| `swarm` | Main API server — REST + WebSocket |
| `mqtt_intake` | MQTT listener → task queue bridge |
| `todo_worker` | Background task processor |
| `mcp_todo_server` | MCP JSON-RPC server for AI tool calls |
| `project_worker` | Project classification service |
| `train_flappy` | RL training runner |
| `test_mcp_todo_publish` | Dev tool for testing task publishing |

The `todo_worker` uses a semaphore-limited concurrency model (`MAX_CONCURRENT_TASKS = 1` by default, configurable) with exponential backoff on failures, per-priority metrics, and health status published to `health/todo_worker`.

---

## Deployment

Production runs on AWS EC2 (`eaws`) behind nginx and pm2. Several build/deploy paths are supported:

```bash
# macOS → EC2 cross-compilation
./build_macos_to_ec2.sh

# WSL build
./build_direct_wsl.sh

# Deploy to EC2
./deploy_to_ec2.sh
```

See `CROSS_COMPILATION.md`, `MACOS_TO_EC2.md`, `WSL_BUILD.md`, `WINDOWS_BUILD.md`, and `EC2_BUILD_README.md` for platform-specific guides. See `DOCKER.md` for container deployment.

---

## Contributing

Open issues. Pull requests welcome. If you're adding an agent, implement the `Agent` trait, register it in the `AgentRegistry`, and add a feature flag. If you're touching MQTT topic structure, update the topic map in this README.

The one rule: don't route UI operations through MCP, and don't call REST API from AI chat responses. See [CLAUDE.md](../../CLAUDE.md) for the full architecture contract.

## License

MIT. Tinker responsibly.

---

## Subrepo Structure

This project is a cog in the grand machine of [Madness Interactive](https://github.com/DanEdens/madness_interactive) — a monorepo of mad science, automation, and carefully version-controlled chaos. Each subproject is modular, each is interconnected, and together they form something greater than the sum of their parts.[^3]

[^3]: Or at least noisier. The MQTT broker never sleeps.

**Ferrum Corde.**
