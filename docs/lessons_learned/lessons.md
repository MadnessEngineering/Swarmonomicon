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

When using AI to enhance todo descriptions, it's important to preserve both the original and enhanced versions:

1. Store both descriptions in the data model:
   - Original description for user reference and exact matching
   - Enhanced description for additional context and details

2. Use the appropriate description based on context:
   - Show original description in lists and basic views
   - Use enhanced description for detailed views or AI processing

3. Make AI enhancement optional:
   - Allow tasks to be created without enhancement
   - Store enhanced description as Option<String>
   - Only enhance at appropriate system boundaries (e.g., TodoTool)

This approach maintains compatibility with existing code while adding AI enhancement capabilities. 
