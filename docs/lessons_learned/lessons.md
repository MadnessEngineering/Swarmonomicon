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
