# TODO List

## High Priority

### Fix Agent Registration System
1. [x] Update `AgentRegistry` implementation
   - [x] Review internal storage type (`Arc<Box<dyn Agent + Send + Sync>>`)
   - [x] Consider if we need both `Arc` and `Box`
   - [x] Document the wrapping pattern decisions

2. [x] Fix `TransferService` implementation
   - [x] Review how it interacts with `AgentRegistry`
   - [x] Consider if we need `Arc<RwLock<AgentRegistry>>` or if `Arc<AgentRegistry>` is sufficient
   - [x] Update methods to handle concurrent access correctly

3. [x] Update agent registration in `swarm.rs`
   - [x] Remove direct `Arc<RwLock>` wrapping of agents
   - [x] Let `AgentRegistry::register` handle the wrapping
   - [x] Update the test cases to match this pattern

4. [x] Fix type mismatches
   - [x] Update `get_agent` to return correct type
   - [x] Fix `registry.read()` vs `registry.write()` usage
   - [x] Ensure consistent agent access patterns

### AI Integration
1. [x] Implement centralized AI client
   - [x] Add configurable endpoint support
   - [x] Add model selection support
   - [x] Implement conversation history
   - [x] Add system prompt handling

2. [x] Update agents to use AI client
   - [x] Update GreeterAgent
   - [x] Update GitAssistantAgent
   - [x] Add conversation history support
   - [x] Implement proper error handling

3. [ ] Enhance AI communication
   - [ ] Add retry mechanism for failed requests
   - [ ] Implement request timeout handling
   - [ ] Add rate limiting
   - [ ] Improve error messages

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
   - [x] Consider if we need write locks for read-only operations
   - [ ] Add timeouts to prevent deadlocks
   - [ ] Handle lock poisoning cases

### Test Coverage
1. [ ] Fix failing integration tests
   - [ ] Review `test_haiku_git_integration`
   - [ ] Ensure proper cleanup in tests
   - [ ] Add more granular test cases

2. [ ] Add unit tests
   - [x] Test agent registration
   - [x] Test message routing
   - [ ] Test state transitions
   - [x] Test AI client functionality

### Fix Failing Tests
1. [ ] Fix `test_invalid_transfer` in `agents::greeter`
   - [ ] Review the expected behavior for invalid transfers
   - [ ] Update the test to match the current implementation
   - [ ] Consider if the failure is due to a bug or outdated test

2. [ ] Fix `test_haiku_generation` in `agents::haiku`
   - [ ] Investigate the overflow error
   - [ ] Review the haiku generation logic for potential bugs
   - [ ] Update the test to handle edge cases

3. [ ] Fix `test_state_transitions` in `agents::haiku`
   - [ ] Review the assertion failure
   - [ ] Ensure the state is properly initialized
   - [ ] Update the test to match the expected state transitions

4. [ ] Fix `test_agent_workflow` in `agents`
   - [ ] Investigate the overflow error
   - [ ] Review the agent workflow logic for potential bugs
   - [ ] Update the test to handle edge cases

5. [ ] Fix `test_agent_transfer` in `agents::transfer`
   - [ ] Review the assertion failure
   - [ ] Ensure the transfer logic is properly implemented
   - [ ] Update the test to match the expected behavior

## Medium Priority

### Documentation
1. [x] Add inline documentation for public APIs
2. [x] Create usage examples
3. [x] Document concurrency patterns
4. [x] Document AI integration

### Features
1. [ ] Implement proper error handling for agent transfers
2. [ ] Add timeout mechanism for long-running operations
3. [ ] Implement agent state persistence
4. [ ] Add configuration file support

### Implement Missing Features
1. [ ] Identify any unimplemented features
   - [ ] Review the project requirements
   - [ ] Create a list of missing features
   - [ ] Prioritize the implementation based on dependencies

2. [ ] Implement the missing features
   - [ ] Follow the existing code patterns
   - [ ] Write unit tests for the new features
   - [ ] Ensure proper error handling and edge case coverage

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

### Refactor and Optimize
1. [ ] Review the codebase for potential refactoring
   - [ ] Identify any code duplication
   - [ ] Look for opportunities to improve performance
   - [ ] Consider improving the code organization and modularity

2. [ ] Perform the identified refactorings
   - [ ] Create separate branches for each refactoring
   - [ ] Ensure the tests pass after each refactoring
   - [ ] Update the documentation if necessary

### Enhance Test Coverage
1. [ ] Review the existing test coverage
   - [ ] Identify any missing test cases
   - [ ] Consider adding more edge case scenarios
   - [ ] Look for opportunities to improve test organization

2. [ ] Implement the identified test enhancements
   - [ ] Write new test cases
   - [ ] Refactor existing tests if necessary
   - [ ] Ensure all tests pass consistently

## Completed
- [x] Create initial agent system
- [x] Implement basic message routing
- [x] Add git operations support
- [x] Add haiku generation
- [x] Implement project initialization
- [x] Create `AgentWrapper` to handle type complexity
- [x] Update `AgentRegistry` to use wrapper type
- [x] Update agent registration to use new pattern
- [x] Implement centralized AI client
- [x] Add conversation history support
- [x] Update agents to use AI client
- [x] Fix concurrent access patterns

## Next Steps
1. Prioritize fixing the failing tests to ensure the existing functionality is working as expected.
2. Review the partially implemented features and complete their implementation.
3. Identify and implement any missing features based on the project requirements.
4. Refactor and optimize the codebase for better maintainability and performance.
5. Enhance the test coverage to ensure robustness and catch potential bugs.
