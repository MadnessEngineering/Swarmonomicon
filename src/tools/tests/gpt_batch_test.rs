use super::*;
use std::env;
use tokio;

#[tokio::test]
async fn test_gpt_batch_tool_creation() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    assert!(tool.pending_requests.lock().await.is_empty());
}

#[tokio::test]
async fn test_batch_request_submission() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    
    let request = BatchRequest {
        messages: vec!["Test message".to_string()],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };

    let response = tool.submit_request(request).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_tool_executor_interface() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    
    let mut params = HashMap::new();
    params.insert("prompt".to_string(), "Test prompt".to_string());
    params.insert("model".to_string(), "gpt-4".to_string());
    params.insert("temperature".to_string(), "0.7".to_string());
    params.insert("max_tokens".to_string(), "100".to_string());

    let response = tool.execute(params).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_batch_processing() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    
    let requests: Vec<_> = (0..5).map(|i| BatchRequest {
        messages: vec![format!("Test message {}", i)],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
    }).collect();

    let responses: Vec<_> = futures::future::join_all(
        requests.into_iter().map(|req| tool.submit_request(req))
    ).await;

    assert!(responses.iter().all(|r| r.is_ok()));
    assert_eq!(responses.len(), 5);
}

#[tokio::test]
async fn test_invalid_api_key() {
    let tool = GPTBatchTool::new("invalid-key".to_string());
    
    let request = BatchRequest {
        messages: vec!["Test message".to_string()],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };

    let response = tool.submit_request(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_batch_window_timing() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    
    // Submit a single request
    let request = BatchRequest {
        messages: vec!["Test message".to_string()],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };

    let start = Instant::now();
    let response = tool.submit_request(request).await;
    let duration = start.elapsed();

    // The request should be processed after the batch window
    assert!(duration >= Duration::from_millis(BATCH_WINDOW_MS));
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_max_batch_size() {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let tool = GPTBatchTool::new(api_key);
    
    let requests: Vec<_> = (0..MAX_BATCH_SIZE + 5).map(|i| BatchRequest {
        messages: vec![format!("Test message {}", i)],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(100),
    }).collect();

    let responses: Vec<_> = futures::future::join_all(
        requests.into_iter().map(|req| tool.submit_request(req))
    ).await;

    assert_eq!(responses.len(), MAX_BATCH_SIZE + 5);
    assert!(responses.iter().all(|r| r.is_ok()));
} 
