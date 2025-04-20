use super::*;
use crate::tools::GPTBatchTool;
use async_openai::types::ChatCompletionFunctions;
use serde_json::json;

// Import our new test module
pub mod mcp_todo_client_test;

#[tokio::test]
async fn test_gpt_batch_tool_integration() {
    let registry = ToolRegistry::create_default_tools().await.unwrap();
    
    let tool = Tool {
        name: "gpt_batch".to_string(),
        description: "GPT-4 batch processing tool".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("prompt".to_string(), "Tell me a joke".to_string());
            params.insert("model".to_string(), "gpt-4".to_string());
            params
        },
    };

    let result = registry.execute(&tool, tool.parameters.clone()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gpt_batch_tool_with_functions_integration() {
    let registry = ToolRegistry::create_default_tools().await.unwrap();
    
    let functions = vec![ChatCompletionFunctions {
        name: "get_weather".to_string(),
        description: Some("Get the current weather".to_string()),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                }
            },
            "required": ["location"]
        }),
    }];

    let tool = Tool {
        name: "gpt_batch".to_string(),
        description: "GPT-4 batch processing tool".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("prompt".to_string(), "What's the weather in San Francisco?".to_string());
            params.insert("model".to_string(), "gpt-4".to_string());
            params.insert("functions".to_string(), serde_json::to_string(&functions).unwrap());
            params.insert("function_call".to_string(), "get_weather".to_string());
            params
        },
    };

    let result = registry.execute(&tool, tool.parameters.clone()).await;
    assert!(result.is_ok());
} 
