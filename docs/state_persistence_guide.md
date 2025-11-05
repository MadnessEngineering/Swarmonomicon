# State Persistence Guide

## Overview

The Swarmonomicon state persistence system allows agents to save and restore their state across restarts, providing durability and recovery capabilities.

## Architecture

### Components

1. **PersistedState** - Core state representation stored in MongoDB
2. **StateTransition** - Records state changes over time
3. **StatePersistence** - Trait for basic save/load operations
4. **StateValidator** - Validates state data and transitions
5. **StateRecovery** - Checkpoint and replay capabilities
6. **AgentStatePersistenceHelper** - Convenience wrapper for agents

### Storage Layer

State is stored in MongoDB with three collections:
- `agent_states` - Current and historical agent states
- `state_transitions` - Transition log for replay
- `state_checkpoints` - Recovery points for rollback

## Using State Persistence in Agents

### Basic Setup

```rust
use crate::state::agent_persistence::AgentStatePersistenceHelper;
use crate::state::persistence::MongoPersistence;
use mongodb::Client;

// Initialize persistence
let client = Client::with_uri_str("mongodb://localhost:27017").await?;
let persistence = MongoPersistence::new(&client).await?;
let helper = AgentStatePersistenceHelper::new(persistence);
```

### Saving Agent State

```rust
// Create state manager with agent ID
let state_manager = AgentStateManager::new(state_machine)
    .with_agent_id("my_agent".to_string());

// Save state with conversation context and metadata
helper.save_agent_state(
    &state_manager,
    conversation_history,
    metadata_map,
).await?;
```

### Loading Agent State

```rust
// Load state on agent initialization
if let Some((state_name, conversation, metadata)) =
    helper.load_agent_state("my_agent").await? {

    // Restore agent state
    state_manager.set_state(state_name);
    self.conversation_history = conversation;
    // Apply metadata as needed
}
```

### Recording Transitions

```rust
// Before transitioning
let old_state = state_manager.get_current_state_name().unwrap();

// Perform transition
state_manager.transition("user_input");

// Record the transition
let new_state = state_manager.get_current_state_name().unwrap();
helper.record_transition(
    "my_agent",
    old_state,
    new_state,
    "user_input",
    true,  // success
    None,  // no error
).await?;
```

## Validation

The system validates:
- Required fields (agent_id, state_name)
- Version numbers (non-negative, incrementing)
- Timestamps (logical ordering)
- Transition legality (non-empty, no self-transitions by default)
- State data structure

## Recovery Features

### Checkpoints

```rust
// Create checkpoint before risky operation
helper.persistence.create_checkpoint(&current_state).await?;

// Rollback if needed
if let Some(checkpoint) = helper.persistence.rollback_to_checkpoint("my_agent").await? {
    // Restore from checkpoint
}
```

### Transition Replay

```rust
// Replay transitions from a specific version
let restored_state = helper.persistence
    .replay_transitions("my_agent", version_number)
    .await?;
```

## AgentStateManager Enhancements

The `AgentStateManager` now includes:
- `with_agent_id(id)` - Set agent identifier
- `get_version()` - Get current version number
- `get_agent_id()` - Get agent identifier
- `set_state(name)` - Manually set state (increments version)

Version numbers automatically increment on transitions.

## Best Practices

1. **Initialize with Agent ID**: Always call `.with_agent_id()` when creating state managers
2. **Save Periodically**: Persist state after significant operations
3. **Record All Transitions**: Log transitions for audit trail and replay
4. **Validate Before Save**: The system validates automatically, but check critical data
5. **Use Checkpoints**: Create checkpoints before complex multi-step operations
6. **Handle Load Failures**: Always have default state if persistence fails

## Example: Full Agent Integration

```rust
pub struct MyAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
    persistence: Option<AgentStatePersistenceHelper<MongoPersistence>>,
    conversation_history: Vec<Message>,
}

impl MyAgent {
    pub async fn new(config: AgentConfig, mongo_client: Option<&Client>) -> Result<Self> {
        let mut state_manager = AgentStateManager::new(config.state_machine.clone())
            .with_agent_id(config.name.clone());

        let persistence = if let Some(client) = mongo_client {
            let persistence = MongoPersistence::new(client).await?;
            let helper = AgentStatePersistenceHelper::new(persistence);

            // Try to restore state
            if let Some((state, conversation, _)) =
                helper.load_agent_state(&config.name).await? {
                state_manager.set_state(state);
                let conversation_history = conversation;
            }

            Some(helper)
        } else {
            None
        };

        Ok(Self {
            config,
            state_manager,
            persistence,
            conversation_history: Vec::new(),
        })
    }

    async fn process_with_persistence(&mut self, message: Message) -> Result<Message> {
        let old_state = self.state_manager.get_current_state_name()
            .unwrap_or("default").to_string();

        // Process message...
        let response = self.do_processing(&message).await?;

        // Record transition if state changed
        if let Some(new_state) = self.state_manager.get_current_state_name() {
            if new_state != old_state {
                if let Some(helper) = &self.persistence {
                    helper.record_transition(
                        &self.config.name,
                        &old_state,
                        new_state,
                        "message_processed",
                        true,
                        None
                    ).await?;
                }
            }
        }

        // Save state periodically
        if let Some(helper) = &self.persistence {
            helper.save_agent_state(
                &self.state_manager,
                self.conversation_history.clone(),
                HashMap::new(),
            ).await?;
        }

        Ok(response)
    }
}
```

## Testing

State persistence includes comprehensive tests:
- `test_mongo_persistence` - Basic CRUD operations
- `test_state_recovery` - Checkpoint and replay
- `test_state_validation` - Validation rules
- `test_agent_persistence_helper` - Helper functions

Run tests with MongoDB running:
```bash
# Start MongoDB
docker run -d -p 27017:27017 mongo

# Run tests
cargo test state_persistence
```

## Future Enhancements

Planned improvements:
- [ ] State migration system for schema changes
- [ ] Automatic checkpoint scheduling
- [ ] State compression for large histories
- [ ] Distributed state replication
- [ ] State versioning and rollback UI
- [ ] Performance metrics collection
- [ ] State garbage collection
