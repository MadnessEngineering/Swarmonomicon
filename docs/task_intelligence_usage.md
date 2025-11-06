# Task Intelligence System - Usage Guide

## Overview

The Swarmonomicon Task Intelligence system provides ML-powered task management capabilities:
- **Priority Prediction**: Learn optimal priorities from historical patterns
- **Smart Decomposition**: Break complex tasks into manageable subtasks
- **Dependency Learning**: Discover task relationships automatically
- **Time Prediction**: Estimate execution time based on history

## Quick Start

### 1. Basic Usage (Without Intelligence)

```rust
use swarmonomicon::agents::task_intelligence::SmartTodoList;

// Create a regular todo list without ML
let smart_list = SmartTodoList::without_intelligence().await?;

// Use like a normal TodoList
let task = TodoTask { /* ... */ };
smart_list.add_smart_task(task).await?;
```

### 2. Full Intelligence Setup

```rust
use swarmonomicon::agents::task_intelligence::{
    SmartTodoList,
    TaskIntelligenceConfig,
};
use mongodb::Client;

// Connect to MongoDB
let mongo_client = Client::with_uri_str("mongodb://localhost:27017").await?;

// Create config with all intelligence features enabled
let config = TaskIntelligenceConfig::new(mongo_client);

// Create smart todo list
let smart_list = SmartTodoList::new(config).await?;

// Add task with automatic ML enhancements
let task = TodoTask {
    id: Uuid::new_v4().to_string(),
    description: "Implement user authentication system".to_string(),
    priority: TaskPriority::Inital, // Will be predicted by ML
    target_agent: "backend".to_string(),
    /* ... */
};

smart_list.add_smart_task(task).await?;
// ML automatically:
// - Predicts optimal priority
// - Estimates execution time
// - Finds dependencies
```

## Features

### Automatic Priority Prediction

The system learns optimal priorities from historical task completion patterns:

```rust
// Task with vague priority gets ML prediction
let task = TodoTask {
    description: "Fix critical security vulnerability in auth".to_string(),
    priority: TaskPriority::Inital, // Initial/unknown
    /* ... */
};

smart_list.add_smart_task(task).await?;
// ML predicts: TaskPriority::High (based on "critical", "security")
```

**How it works:**
- Uses k-NN ML to find similar historical tasks
- Extracts features: keywords, complexity, urgency indicators
- Predicts priority with confidence scoring
- Falls back to heuristics when insufficient data

### Smart Task Decomposition

Break complex tasks into subtasks automatically:

```rust
let complex_task = TodoTask {
    description: "Implement new user dashboard feature with analytics".to_string(),
    priority: TaskPriority::High,
    /* ... */
};

// Decompose and create subtasks
let subtasks = smart_list.decompose_and_add(complex_task).await?;

// Creates:
// 1. Design and plan: Implement new user dashboard feature...
// 2. Implement: Implement new user dashboard feature...
// 3. Test: Implement new user dashboard feature...
// 4. Document: Implement new user dashboard feature...
```

**Decomposition Strategies:**
- **ByPhase**: Design → Implement → Test → Document
- **ByComponent**: Split by system components
- **ByIncrement**: MVP → Enhance → Polish
- **ByLayer**: Database → Backend → Frontend → Testing

### Dependency Discovery

Automatically find task dependencies:

```rust
let task = TodoTask {
    description: "Deploy new API to production".to_string(),
    /* ... */
};

smart_list.add_smart_task(task).await?;

// ML discovers dependencies and adds to notes:
// "Dependencies:
//  - Depends on: Task involving test, verify (confidence: 95%)
//  - Depends on: Task involving implement, code (confidence: 90%)"
```

**Learned Dependency Rules:**
- Design before Implementation (90% confidence)
- Implementation before Testing (95% confidence)
- Testing before Deployment (95% confidence)
- API before Frontend (85% confidence)
- Database before API (80% confidence)

### Execution Time Prediction

Predict how long tasks will take:

```rust
use swarmonomicon::agents::task_intelligence::TaskIntelligenceService;

let service = TaskIntelligenceService::new(config).await?;

// Predict duration
let prediction = service.predict_execution_time(&task).await?;

println!("Estimated: {} minutes", prediction.estimated_minutes());
println!("Confidence: {:.1}%", prediction.confidence * 100.0);
println!("Range: {} - {} minutes",
         prediction.min_seconds / 60,
         prediction.max_seconds / 60);
```

**How it works:**
- Analyzes similar historical tasks
- Adjusts for complexity and priority
- Provides confidence intervals
- Tracks agent-specific performance profiles

### Learning from Execution

Record task outcomes to improve predictions:

```rust
use swarmonomicon::agents::task_intelligence::TaskOutcome;

// Task completed
smart_list.record_completion(
    &task,
    TaskOutcome::Success,
    duration_seconds
).await?;

// ML learns:
// - Priority accuracy improves
// - Time predictions refine
// - Dependency patterns strengthen
// - Decomposition strategies adapt
```

### Training from History

Manually train ML models from historical data:

```rust
// Train all components
smart_list.train_models().await?;

// Check accuracy
let stats = smart_list.get_intelligence_stats().await?;
println!("Total tasks analyzed: {}", stats.total_tasks);
println!("Priority accuracy: {:.1}%", stats.priority_accuracy * 100.0);
println!("Avg completion time: {:.1} min", stats.avg_completion_time / 60.0);
println!("Dependency rules learned: {}", stats.dependency_rules);
```

## Configuration

### TaskIntelligenceConfig Options

```rust
let mut config = TaskIntelligenceConfig::new(mongo_client);

// Disable specific features
config.enable_priority_prediction = false;
config.enable_decomposition = false;
config.enable_dependency_learning = false;
config.enable_time_prediction = false;

// Set learning threshold (minimum tasks before ML activates)
config.learning_threshold = 20; // Default: 20 tasks

let smart_list = SmartTodoList::new(config).await?;
```

### Feature Control

```rust
// Fully enabled
let config = TaskIntelligenceConfig::new(mongo_client);

// Partially enabled
let mut config = TaskIntelligenceConfig::new(mongo_client);
config.enable_decomposition = false; // No task decomposition

// Disabled (acts like regular TodoList)
let config = TaskIntelligenceConfig::disabled();
```

## Architecture

```
┌──────────────────────────────────────┐
│      SmartTodoList                   │
├──────────────────────────────────────┤
│  ┌────────────────────────────────┐  │
│  │  TaskHistory                   │  │ ← MongoDB
│  │  (records all task executions) │  │
│  └────────────────────────────────┘  │
│           ↓         ↓         ↓       │
│  ┌─────────┐  ┌──────────┐  ┌──────┐│
│  │Priority │  │Decomposer│  │Dep.  ││
│  │Predictor│  │ (patterns)│  │Learner││
│  │ (k-NN)  │  │          │  │(rules)││
│  └─────────┘  └──────────┘  └──────┘│
│       ↓            ↓           ↓      │
│  ┌─────────┐                         │
│  │  Time   │                         │
│  │Predictor│                         │
│  │(history)│                         │
│  └─────────┘                         │
│       ↓                               │
│  Enhanced TodoTask                    │
└──────────────────────────────────────┘
```

## Integration with Existing Code

### Drop-in Replacement

SmartTodoList is a drop-in replacement for TodoList:

```rust
// Old code
let todo_list = TodoList::new().await?;
todo_list.add_task(task).await?;

// New code (with intelligence)
let smart_list = SmartTodoList::new(config).await?;
smart_list.add_smart_task(task).await?;

// Access underlying TodoList if needed
smart_list.todo_list().get_all_tasks().await?;
```

### With TodoProcessor

```rust
use swarmonomicon::types::TodoProcessor;

struct MyProcessor {
    smart_list: SmartTodoList,
}

#[async_trait::async_trait]
impl TodoProcessor for MyProcessor {
    async fn process_task(&self, task: TodoTask) -> Result<Message> {
        let start = Instant::now();

        // Process task...
        let result = self.do_work(&task).await?;

        // Record execution for learning
        let duration = start.elapsed().as_secs() as i64;
        self.smart_list.record_completion(
            &task,
            TaskOutcome::Success,
            duration
        ).await?;

        Ok(result)
    }

    fn get_todo_list(&self) -> &TodoList {
        self.smart_list.todo_list()
    }

    // ...
}
```

## Best Practices

### 1. Always Record Outcomes

```rust
// Good - ML learns from every task
let start = Instant::now();
match process_task(&task).await {
    Ok(_) => {
        let duration = start.elapsed().as_secs() as i64;
        smart_list.record_completion(&task, TaskOutcome::Success, duration).await?;
    }
    Err(_) => {
        let duration = start.elapsed().as_secs() as i64;
        smart_list.record_completion(&task, TaskOutcome::Failure, duration).await?;
    }
}

// Bad - No learning happens
process_task(&task).await?;
```

### 2. Train Periodically

```rust
// Train models after collecting sufficient data
if task_count % 100 == 0 {
    smart_list.train_models().await?;
}
```

### 3. Monitor Learning Progress

```rust
// Check accuracy regularly
let stats = smart_list.get_intelligence_stats().await?;

if stats.priority_accuracy < 0.6 {
    tracing::warn!("Priority prediction accuracy low: {:.1}%",
                   stats.priority_accuracy * 100.0);
    // Consider retraining or collecting more data
}
```

### 4. Use Decomposition Wisely

```rust
// Check if task should be decomposed
use swarmonomicon::agents::task_intelligence::TaskDecomposer;

let decomposer = /* ... */;

if decomposer.should_decompose(&task).await? {
    smart_list.decompose_and_add(task).await?;
} else {
    smart_list.add_smart_task(task).await?;
}
```

### 5. Respect Confidence Scores

```rust
// Only act on high-confidence predictions
let prediction = service.predict_priority(&description).await?;

if let Some(predicted) = prediction {
    let confidence = /* get from features */;
    if confidence > 0.7 {
        task.priority = predicted;
    } else {
        // Use manual priority or heuristics
    }
}
```

## Complete Example

```rust
use swarmonomicon::agents::task_intelligence::*;
use swarmonomicon::types::*;
use mongodb::Client;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup MongoDB
    let mongo = Client::with_uri_str("mongodb://localhost:27017").await?;

    // Create smart todo list
    let config = TaskIntelligenceConfig::new(mongo);
    let smart_list = SmartTodoList::new(config).await?;

    // Create a complex task
    let task = TodoTask {
        id: Uuid::new_v4().to_string(),
        description: "Implement new user authentication system with OAuth2".to_string(),
        priority: TaskPriority::Inital, // Let ML predict
        project: Some("backend".to_string()),
        target_agent: "backend_agent".to_string(),
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now().timestamp(),
        /* ... */
    };

    // Decompose if complex
    let subtasks = smart_list.decompose_and_add(task).await?;
    println!("Created {} subtasks", subtasks.len());

    // Process first subtask
    let first_task = &subtasks[0];
    let start = Instant::now();

    // ... do work ...

    // Record success
    let duration = start.elapsed().as_secs() as i64;
    smart_list.record_completion(
        first_task,
        TaskOutcome::Success,
        duration
    ).await?;

    // Check learning progress
    let stats = smart_list.get_intelligence_stats().await?;
    println!("Stats: {:?}", stats);

    Ok(())
}
```

## Performance Considerations

- **MongoDB**: All operations are async and indexed
- **Memory**: Models cached in-memory after loading
- **CPU**: k-NN prediction is O(n) but fast for typical datasets
- **Learning Threshold**: System waits for 20+ tasks before activating ML

## Troubleshooting

### "Not enough data for ML"

ML requires minimum 20 tasks. Check:
```rust
let stats = smart_list.get_intelligence_stats().await?;
if stats.total_tasks < 20 {
    println!("Need {} more tasks for ML", 20 - stats.total_tasks);
}
```

### "Priority prediction not working"

Check configuration:
```rust
if !config.enable_priority_prediction {
    println!("Priority prediction is disabled in config");
}
```

### "Decomposition not happening"

Ensure task is complex enough:
```rust
let features = TaskFeatures::extract(&task.description);
if features.estimated_complexity < 0.6 {
    println!("Task too simple for decomposition");
}
```

## Future Enhancements

Planned features:
- [ ] Neural network priority prediction
- [ ] Collaborative filtering (learn from similar users)
- [ ] Transfer learning between projects
- [ ] Real-time A/B testing of strategies
- [ ] Explainable AI (why this priority/decomposition?)
- [ ] Task clustering and pattern mining

---

**Ferrum Corde!** ⚙️
