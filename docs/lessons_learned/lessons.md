## Code Integration and Error Handling
- When integrating with existing tools like TodoTool, ensure proper error handling and type conversion using anyhow::Error
- Be mindful of async/await usage - not all functions need to be async even in an async context
- When using traits (like ToolExecutor), remember to import them into scope
- Error handling in Rust requires careful attention to error type conversion, especially when dealing with multiple error types 

## Ollama Integration Lessons
- Always check model availability before attempting to use it
- Provide clear error messages with context about what failed
- Use tracing macros (debug, warn, error) for better debugging and monitoring
- Structure prompts with clear separators for better model understanding
- Keep model constants and command names in constants for easy updates
- Add proper error handling for both command execution and output parsing
- Include model management (checking availability, pulling if needed)
- Add comprehensive tests for prompt formatting and model availability

## Best Practices for AI Integration
- Default to efficient models that balance size and capability
- Implement proper error recovery and fallback mechanisms
- Structure prompts consistently with clear role separation
- Add logging at appropriate levels for debugging
- Make model selection configurable but with sensible defaults
- Test both happy path and error conditions
- Consider resource constraints when selecting default models
- Document model requirements and dependencies 

## Concurrent Processing and Rate Limiting
- Use semaphores to control concurrent task processing and prevent system overload
- Implement separate rate limits for different resource types (e.g., tasks vs AI calls)
- Process long-running operations in separate tokio tasks for better responsiveness
- Use Arc for sharing resources safely between concurrent tasks
- Leverage RAII for automatic cleanup of resources (e.g., semaphore permits)
- Add proper error handling for permit acquisition failures
- Monitor and log task processing status for debugging
- Consider system resources and external service limits when setting concurrency limits
- Use structured logging to track task lifecycle across concurrent operations
- Implement graceful degradation when resource limits are reached 

## Todo System

### Handling AI-Enhanced Descriptions

When using AI to enhance todo descriptions, it's important to follow these principles:

1. Separation of Concerns:
   - Infrastructure level (API/worker) - No AI enhancement, just task creation
   - Agent level - Responsible for AI enhancement during task processing
   - TodoList - Provides utilities for both enhanced and non-enhanced task creation

2. Data Storage:
   - Original description is always preserved
   - Enhanced description is optional (Option<String>)
   - Both versions are stored in MongoDB
   - Database name is configurable via RTK_MONGO_DB env var

3. Enhancement Flow:
   - Tasks are initially created with no enhancement
   - Agents can enhance tasks during processing
   - Enhancement includes technical details, scope, and impact
   - Failed enhancements gracefully fallback to original description

4. Testing Considerations:
   - Use separate test database (swarmonomicon_test)
   - Mock AI clients for predictable test behavior
   - Verify both enhanced and non-enhanced paths
   - Clean up test data after each test run

5. Best Practices:
   - Use create_task_with_enhancement for consistent task creation
   - Handle AI enhancement failures gracefully
   - Log enhancement attempts and results
   - Keep original description for exact matching
   - Use enhanced description for rich context

This approach maintains system integrity while providing flexible AI enhancement capabilities. 
