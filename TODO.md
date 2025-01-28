# TODO List

## High Priority

### Fix Agent Registration System
1. [x] Update `AgentRegistry` implementation
   - [x] Review internal storage type (`Arc<Box<dyn Agent + Send + Sync>>`)
   - [x] Consider if we need both `Arc` and `Box`
   - [x] Document the wrapping pattern decisions

2. [ ] Fix `TransferService` implementation
   - [x] Review how it interacts with `AgentRegistry`
   - [ ] Consider if we need `Arc<RwLock<AgentRegistry>>` or if `Arc<AgentRegistry>` is sufficient
   - [ ] Update methods to handle concurrent access correctly

3. [x] Update agent registration in `swarm.rs`
   - [x] Remove direct `Arc<RwLock>` wrapping of agents
   - [x] Let `AgentRegistry::register` handle the wrapping
   - [x] Update the test cases to match this pattern

4. [ ] Fix type mismatches
   - [x] Update `get_agent` to return correct type
   - [ ] Fix `registry.read()` vs `registry.write()` usage
   - [ ] Ensure consistent agent access patterns

### Current Issues to Fix
1. [ ] Fix lifetime issue in `AgentWrapper::get_mut`
   ```rust
   // Current issue:
   pub fn get_mut(&mut self) -> Option<&mut (dyn Agent + Send + Sync + '_)>
   ```

2. [ ] Fix trait method access in `routes.rs`
   - [ ] Import `Agent` trait where needed
   - [ ] Ensure proper trait bounds on generic parameters

3. [ ] Review locking patterns in `TransferService`
   - [ ] Consider if we need write locks for read-only operations
   - [ ] Add timeouts to prevent deadlocks
   - [ ] Handle lock poisoning cases

### Test Coverage
1. [ ] Fix failing integration tests
   - [ ] Review `test_haiku_git_integration`
   - [ ] Ensure proper cleanup in tests
   - [ ] Add more granular test cases

2. [ ] Add unit tests
   - [x] Test agent registration
   - [ ] Test message routing
   - [ ] Test state transitions

## Medium Priority

### Documentation
1. [ ] Add inline documentation for public APIs
2. [ ] Create usage examples
3. [ ] Document concurrency patterns

### Features
1. [ ] Implement proper error handling for agent transfers
2. [ ] Add timeout mechanism for long-running operations
3. [ ] Implement agent state persistence
4. [ ] Add configuration file support

## Low Priority

### Improvements
1. [ ] Add metrics collection
2. [ ] Implement logging system
3. [ ] Add health checks for agents
4. [ ] Create admin interface

### Technical Debt
1. [ ] Reduce code duplication in agent implementations
2. [ ] Optimize lock patterns
3. [ ] Implement proper shutdown sequence
4. [ ] Add proper error types instead of using Box<dyn Error>

## Completed
- [x] Create initial agent system
- [x] Implement basic message routing
- [x] Add git operations support
- [x] Add haiku generation
- [x] Implement project initialization
- [x] Create `AgentWrapper` to handle type complexity
- [x] Update `AgentRegistry` to use wrapper type
- [x] Update agent registration to use new pattern

## Next Steps
1. Create a wrapper type for agents to handle the type complexity:
   ```rust
   pub struct AgentWrapper {
       inner: Arc<Box<dyn Agent + Send + Sync>>,
   }
   
   impl AgentWrapper {
       pub fn new<A>(agent: A) -> Self 
       where 
           A: Agent + Send + Sync + 'static 
       {
           Self {
               inner: Arc::new(Box::new(agent))
           }
       }
   }
   ```

2. Update the `AgentRegistry` to use this wrapper:
   ```rust
   pub struct AgentRegistry {
       agents: HashMap<String, AgentWrapper>,
   }
   ```

3. Update the `TransferService` to handle the wrapper:
   ```rust
   impl TransferService {
       pub async fn process_message(&mut self, content: &str) -> Result<Message> {
           let registry = self.registry.read().await;
           if let Some(current_agent) = &self.current_agent {
               if let Some(agent) = registry.get(current_agent) {
                   agent.process_message(content).await
               } else {
                   Err(format!("Current agent '{}' not found", current_agent).into())
               }
           } else {
               Err("No current agent set".into())
           }
       }
   }
   ```
