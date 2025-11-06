# Agent Learning System - Usage Guide

## Overview

The Swarmonomicon learning system enables agents to learn from user interactions, adapt their personality, and optimize routing decisions using reinforcement learning.

## Quick Start

### 1. Basic Setup (Learning Disabled)

```rust
use swarmonomicon::agents::{AgentRegistry, learning_service::*};
use std::sync::Arc;
use tokio::sync::RwLock;

// Create agent registry
let mut registry = AgentRegistry::new();
// ... register your agents ...
let registry = Arc::new(RwLock::new(registry));

// Create service without learning
let service = LearningTransferService::new(
    registry.clone(),
    LearningConfig::default(), // Learning disabled
).await?;

// Use it normally
let response = service.process_message(
    Message::new("Hello!".to_string()),
    None, // No user_id tracking
).await?;
```

### 2. Full Learning Setup

```rust
use mongodb::Client;

// Connect to MongoDB
let mongo_client = Client::with_uri_str("mongodb://localhost:27017").await?;

// Create learning-enabled service
let service = LearningTransferService::new(
    registry.clone(),
    LearningConfig::new(mongo_client),
).await?;

// Process messages with user tracking
let response = service.process_message(
    Message::new("Hello!".to_string()),
    Some("user_123".to_string()), // Track this user!
).await?;
```

## Features

### Smart Routing

Let the system decide which agent to use based on learned patterns:

```rust
// Instead of manual routing...
let response = service.transfer("greeter", "git", message).await?;

// Let the system learn optimal routing!
let response = service.smart_transfer(
    "greeter",
    Message::new("I need help with git commit".to_string()),
    Some("user_123".to_string()),
).await?;
// RL policy automatically routes to best agent based on:
// - User's historical preferences
// - Message intent classification
// - Success patterns
// - Conversation context
```

### Feedback Loop

Provide user satisfaction feedback to improve learning:

```rust
// User interacts
let response = service.process_message(message, Some("user_123".to_string())).await?;

// User provides feedback (0.0 = unhappy, 1.0 = very satisfied)
service.provide_feedback("user_123".to_string(), 0.9).await?;

// System learns:
// - Personality adjusts to user preferences
// - Routing policy gets reinforced
// - Preferences are updated
```

### Personality Adaptation

Agents automatically adapt their personality to each user:

```rust
// After several interactions, the system learns:
// - User prefers detailed responses → verbosity increases
// - User likes technical language → technical level increases
// - User prefers quick responses → patience decreases

// Personality automatically applied in process_message()
// No code changes needed - it just works!
```

### Training from History

Train the RL routing policy from historical data:

```rust
#[cfg(feature = "rl")]
{
    // Train on user's historical interactions
    service.train_routing_policy("user_123", 100).await?;
    // 100 episodes of training from their history

    // Save the trained model
    service.save_models("./models").await?;

    // Later, load it back
    service.load_models("./models").await?;
}
```

### Learning Statistics

Monitor how well the system is learning:

```rust
let stats = service.get_learning_stats("user_123").await?;

println!("Total interactions: {}", stats.total_interactions);
println!("Success rate: {:.1}%", stats.success_rate * 100.0);
println!("Avg satisfaction: {:.2}", stats.avg_satisfaction);
println!("Transfers: {}", stats.total_transfers);
```

## Configuration

### LearningConfig Options

```rust
let mut config = LearningConfig::new(mongo_client);

// Disable specific features
config.enable_routing = false;      // Turn off RL routing
config.enable_personality = false;  // Turn off personality adaptation
config.enable_preference = false;   // Turn off preference learning

let service = LearningTransferService::new(registry, config).await?;
```

### Feature Flags

The system respects cargo feature flags:

```toml
[features]
default = ["greeter-agent", "haiku-agent", "git-agent", "project-agent"]
rl = ["rand", "pixels", "winit", "winit_input_helper", "plotters"]
```

- **With `rl` feature**: Full RL-based routing
- **Without `rl` feature**: Falls back to preference-based routing

## Integration with Existing Code

### Minimal Changes Required

```rust
// Old code (still works!)
let service = TransferService::new(registry);
let response = service.process_message(message).await?;

// New code (with learning)
let service = LearningTransferService::new(registry, config).await?;
let response = service.process_message(message, Some(user_id)).await?;
```

### Drop-in Replacement

`LearningTransferService` provides all the same methods as `TransferService`:

- `process_message()`
- `transfer()`
- `get_agent()`
- `get_current_agent_name()`
- `set_current_agent_name()`

Plus new learning methods:

- `smart_transfer()` - RL-optimized routing
- `provide_feedback()` - Update learning
- `train_routing_policy()` - Train from history
- `save_models()` / `load_models()` - Model persistence
- `get_learning_stats()` - Monitor learning

## Architecture

```
┌──────────────────────────────────────┐
│   LearningTransferService            │
├──────────────────────────────────────┤
│  ┌────────────────────────────────┐  │
│  │  InteractionTracker            │  │ ← MongoDB
│  │  (records all interactions)    │  │
│  └────────────────────────────────┘  │
│           ↓         ↓         ↓       │
│  ┌─────────┐  ┌─────────┐  ┌──────┐  │
│  │ Routing │  │Personal │  │ Pref │  │
│  │ Policy  │  │ Adapter │  │ Pred │  │
│  │  (RL)   │  │ (traits)│  │ (ML) │  │
│  └─────────┘  └─────────┘  └──────┘  │
│       ↓            ↓           ↓      │
│  Smart Routing  Adjusted   Best      │
│                 Prompts    Agent      │
└──────────────────────────────────────┘
```

## Best Practices

### 1. Always Use User IDs

```rust
// Bad - no learning happens
service.process_message(message, None).await?;

// Good - learning accumulates
service.process_message(message, Some(user_id)).await?;
```

### 2. Collect Feedback When Possible

```rust
// Explicit feedback is gold!
service.provide_feedback(user_id, satisfaction_score).await?;

// Implicit feedback (success/failure) is also valuable
// Automatically tracked from interaction outcomes
```

### 3. Train Periodically

```rust
// Train routing policy after collecting data
if interaction_count % 100 == 0 {
    service.train_routing_policy(&user_id, 50).await?;
}
```

### 4. Save Models

```rust
// Save models periodically
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        service.save_models("./models").await.ok();
    }
});
```

### 5. Monitor Learning

```rust
// Check learning progress
let stats = service.get_learning_stats(&user_id).await?;
if stats.success_rate < 0.5 {
    // Maybe retrain or adjust parameters
}
```

## Complete Example

```rust
use swarmonomicon::agents::{
    AgentRegistry,
    learning_service::*,
    greeter::GreeterAgent,
    git_assistant::GitAssistantAgent,
};
use mongodb::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup MongoDB
    let mongo = Client::with_uri_str("mongodb://localhost:27017").await?;

    // Create agent registry
    let mut registry = AgentRegistry::new();
    registry.register("greeter".to_string(), Box::new(
        GreeterAgent::new(/* config */)
    )).await?;
    registry.register("git".to_string(), Box::new(
        GitAssistantAgent::new(/* config */)
    )).await?;
    let registry = Arc::new(RwLock::new(registry));

    // Create learning service
    let service = LearningTransferService::new(
        registry,
        LearningConfig::new(mongo),
    ).await?;

    // User interaction
    let user_id = "user_123".to_string();

    // First message
    let response1 = service.smart_transfer(
        "greeter",
        Message::new("I need help with git".to_string()),
        Some(user_id.clone()),
    ).await?;
    println!("Response: {}", response1.content);

    // Provide feedback
    service.provide_feedback(user_id.clone(), 0.9).await?;

    // Second message (system has learned!)
    let response2 = service.smart_transfer(
        "greeter",
        Message::new("How do I commit changes?".to_string()),
        Some(user_id.clone()),
    ).await?;
    println!("Response: {}", response2.content);

    // Check learning progress
    let stats = service.get_learning_stats(&user_id).await?;
    println!("Success rate: {:.1}%", stats.success_rate * 100.0);

    Ok(())
}
```

## Troubleshooting

### "MongoDB client required when learning is enabled"

Solution: Pass a valid MongoDB client in `LearningConfig`:

```rust
let config = LearningConfig::new(mongo_client);
```

### Learning not happening

Check:
1. Is learning enabled? `config.enabled == true`
2. Are you passing user_id? `Some(user_id)` not `None`
3. Is MongoDB running and accessible?

### RL routing not working

Check:
1. Is `rl` feature enabled? `cargo build --features rl`
2. Is routing enabled in config? `config.enable_routing == true`

## Performance Considerations

- **MongoDB**: Interaction recording is async, doesn't block responses
- **Memory**: Personality profiles cached in-memory, persisted to MongoDB
- **RL Training**: CPU-intensive, run offline or in background
- **Model Size**: Q-tables grow with state/action space, monitor disk usage

## Security Considerations

- **User Privacy**: All interaction history stored - implement data retention policies
- **Model Poisoning**: Validate feedback scores, detect manipulation attempts
- **Access Control**: Protect model files, restrict who can provide feedback
- **Audit Trail**: All interactions logged - useful for debugging and compliance

## Future Enhancements

Planned features:
- [ ] Multi-user collaborative filtering
- [ ] Transfer learning between users
- [ ] Neural network policies (beyond Q-learning)
- [ ] Real-time A/B testing of routing strategies
- [ ] Explainable AI - why was this route chosen?
- [ ] Privacy-preserving federated learning
