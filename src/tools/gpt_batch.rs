use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_openai::{
    config::OpenAIConfig,
    Client,
    types::{
        CreateChatCompletionRequest, ChatCompletionRequestMessage, Role, FunctionCall,
        ChatCompletionFunctions, ChatCompletionFunctionCall, CreateChatCompletionResponse,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        CompletionUsage, ChatChoice,
    },
};
use tracing::{debug, error, info, warn};
use tokio::time::sleep;
use uuid::Uuid;
use serde_json::Value;
use futures;

use crate::tools::ToolExecutor;

const MAX_BATCH_SIZE: usize = 20;
const BATCH_WINDOW_MS: u64 = 1000; // 1 second window for batching
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;
const RATE_LIMIT_REQUESTS: u32 = 3500; // Requests per minute for GPT-4
const RATE_LIMIT_WINDOW_MS: u64 = 60000; // 1 minute
const LONG_RUNNING_JOB_TIMEOUT: Duration = Duration::from_secs(86400); // 24 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub messages: Vec<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u16>,
    pub functions: Option<Vec<ChatCompletionFunctions>>,
    pub function_call: Option<String>,
    pub is_long_running: bool, // Flag for 24h batch mode
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub responses: Vec<String>,
    pub function_calls: Vec<Option<FunctionCall>>,
    pub usage: Option<CompletionUsage>,
    pub choices: Vec<ChatChoice>,
    pub model: String,
    pub system_fingerprint: Option<String>,
}

impl Default for BatchResponse {
    fn default() -> Self {
        Self {
            responses: Vec::new(),
            function_calls: Vec::new(),
            usage: None,
            choices: Vec::new(),
            model: String::new(),
            system_fingerprint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchJobStatus {
    Queued,
    InProgress,
    Completed(BatchResponse),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct BatchJob {
    pub id: String,
    pub request: BatchRequest,
    pub status: Arc<Mutex<BatchJobStatus>>,
    pub created_at: Instant,
}

pub struct GPTBatchTool {
    client: Client<OpenAIConfig>,
    pending_requests: Arc<Mutex<Vec<(BatchRequest, tokio::sync::oneshot::Sender<Result<BatchResponse>>)>>>,
    long_running_jobs: Arc<Mutex<HashMap<String, BatchJob>>>,
    completed_jobs: Arc<Mutex<VecDeque<(String, BatchResponse)>>>,
    last_batch_time: Arc<Mutex<Instant>>,
    request_count: Arc<Mutex<(u32, Instant)>>,
}

impl GPTBatchTool {
    pub fn new(api_key: String) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        let tool = Self {
            client,
            pending_requests: Arc::new(Mutex::new(Vec::new())),
            long_running_jobs: Arc::new(Mutex::new(HashMap::new())),
            completed_jobs: Arc::new(Mutex::new(VecDeque::new())),
            last_batch_time: Arc::new(Mutex::new(Instant::now())),
            request_count: Arc::new(Mutex::new((0, Instant::now()))),
        };

        // Spawn background tasks
        let pending_requests = tool.pending_requests.clone();
        let long_running_jobs = tool.long_running_jobs.clone();
        let completed_jobs = tool.completed_jobs.clone();
        let last_batch_time = tool.last_batch_time.clone();
        let request_count = tool.request_count.clone();
        let client = tool.client.clone();

        // Real-time batch processing
        let request_count1 = request_count.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if let Err(e) = Self::process_batch(&client, &pending_requests, &last_batch_time, &request_count1).await {
                    error!("Error processing batch: {:?}", e);
                }
            }
        });

        // Long-running job processing
        let client = tool.client.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await; // Check every minute
                if let Err(e) = Self::process_long_running_jobs(
                    &client,
                    &long_running_jobs,
                    &completed_jobs,
                    &request_count,
                ).await {
                    error!("Error processing long-running jobs: {:?}", e);
                }
            }
        });

        // Cleanup completed jobs older than 24 hours
        let completed_jobs = tool.completed_jobs.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await; // Check every hour
                let mut jobs = completed_jobs.lock().await;
                while jobs.len() > 1000 { // Keep last 1000 completed jobs
                    jobs.pop_front();
                }
            }
        });

        tool
    }

    async fn check_rate_limit(request_count: &Arc<Mutex<(u32, Instant)>>) -> bool {
        let mut count_data = request_count.lock().await;
        let now = Instant::now();

        if now.duration_since(count_data.1).as_millis() >= RATE_LIMIT_WINDOW_MS as u128 {
            count_data.0 = 0;
            count_data.1 = now;
        }

        if count_data.0 >= RATE_LIMIT_REQUESTS {
            warn!("Rate limit reached, waiting for next window");
            return false;
        }

        count_data.0 += 1;
        true
    }

    async fn process_batch(
        client: &Client<OpenAIConfig>,
        pending_requests: &Arc<Mutex<Vec<(BatchRequest, tokio::sync::oneshot::Sender<Result<BatchResponse>>)>>>,
        last_batch_time: &Arc<Mutex<Instant>>,
        request_count: &Arc<Mutex<(u32, Instant)>>,
    ) -> Result<()> {
        let mut requests = pending_requests.lock().await;
        let now = Instant::now();
        let last_time = *last_batch_time.lock().await;

        if requests.is_empty() || (requests.len() < MAX_BATCH_SIZE && now.duration_since(last_time).as_millis() < BATCH_WINDOW_MS as u128) {
            return Ok(());
        }

        // Take batch of requests
        let batch_size = std::cmp::min(requests.len(), MAX_BATCH_SIZE);
        let batch: Vec<_> = requests.drain(..batch_size).collect();
        *last_batch_time.lock().await = now;
        drop(requests);

        info!("Processing batch of {} requests", batch.len());

        // Process each request in the batch with retries
        for (request, mut response_sender) in batch {
            let mut retry_count = 0;
            let mut batch_response = BatchResponse::default();

            while retry_count < MAX_RETRIES {
                if !Self::check_rate_limit(request_count).await {
                    sleep(Duration::from_millis(RATE_LIMIT_WINDOW_MS)).await;
                    continue;
                }

                let messages: Vec<ChatCompletionRequestMessage> = request.messages
                    .iter()
                    .map(|content| ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessage {
                            content: ChatCompletionRequestUserMessageContent::Text(content.clone()),
                            name: None,
                            role: Role::User,
                        }
                    ))
                    .collect();

                let mut chat_request = CreateChatCompletionRequest::default();
                chat_request.model = request.model.clone();
                chat_request.messages = messages;
                chat_request.temperature = request.temperature;
                chat_request.max_tokens = request.max_tokens;

                if let Some(functions) = &request.functions {
                    chat_request.functions = Some(functions.clone());
                    if let Some(function_call) = &request.function_call {
                        chat_request.function_call = Some(ChatCompletionFunctionCall::Function {
                            name: function_call.clone(),
                        });
                    }
                }

                match client.chat().create(chat_request).await {
                    Ok(response) => {
                        let mut request_count_guard = request_count.lock().await;
                        request_count_guard.0 += 1;
                        drop(request_count_guard);

                        batch_response.choices.push(response.choices[0].clone());
                        batch_response.usage = response.usage;
                        batch_response.model = response.model;
                        batch_response.system_fingerprint = response.system_fingerprint;

                        let responses: Vec<String> = response.choices
                            .iter()
                            .filter_map(|choice| choice.message.content.clone())
                            .collect();

                        let function_calls: Vec<Option<FunctionCall>> = response.choices
                            .iter()
                            .map(|choice| choice.message.function_call.clone())
                            .collect();

                        batch_response.responses = responses;
                        batch_response.function_calls = function_calls;

                        if let Err(e) = response_sender.send(Ok(batch_response)) {
                            error!("Failed to send response: {:?}", e);
                        }
                        break;
                    }
                    Err(e) => {
                        error!("Error in batch request: {:?}", e);
                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            if let Err(e) = response_sender.send(Err(anyhow::anyhow!("Max retries exceeded: {:?}", e))) {
                                error!("Failed to send error response: {:?}", e);
                            }
                            break;
                        }
                        sleep(Duration::from_millis(RETRY_DELAY_MS * (2_u64.pow(retry_count)))).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_long_running_jobs(
        client: &Client<OpenAIConfig>,
        long_running_jobs: &Arc<Mutex<HashMap<String, BatchJob>>>,
        completed_jobs: &Arc<Mutex<VecDeque<(String, BatchResponse)>>>,
        request_count: &Arc<Mutex<(u32, Instant)>>,
    ) -> Result<()> {
        let jobs = long_running_jobs.lock().await;
        let now = Instant::now();

        // Collect jobs to process
        let jobs_to_process: Vec<(String, BatchJob)> = jobs.iter()
            .filter(|(_, job)| {
                let status = futures::executor::block_on(job.status.lock());
                matches!(*status, BatchJobStatus::Queued | BatchJobStatus::InProgress)
            })
            .map(|(id, job)| (id.clone(), job.clone()))
            .collect();

        drop(jobs); // Release the lock

        for (job_id, job) in jobs_to_process {
            // Check for timeout
            if now.duration_since(job.created_at) > LONG_RUNNING_JOB_TIMEOUT {
                let mut jobs = long_running_jobs.lock().await;
                if let Some(job) = jobs.get(&job_id) {
                    *job.status.lock().await = BatchJobStatus::Failed("Job timeout exceeded".to_string());
                }
                continue;
            }

            // Process the job
            if !Self::check_rate_limit(request_count).await {
                continue;
            }

            let messages: Vec<ChatCompletionRequestMessage> = job.request.messages
                .iter()
                .map(|content| ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(content.clone()),
                        name: None,
                        role: Role::User,
                    }
                ))
                .collect();

            let mut chat_request = CreateChatCompletionRequest::default();
            chat_request.model = job.request.model.clone();
            chat_request.messages = messages;
            chat_request.temperature = job.request.temperature;
            chat_request.max_tokens = job.request.max_tokens;

            if let Some(functions) = &job.request.functions {
                chat_request.functions = Some(functions.clone());
                if let Some(function_call) = &job.request.function_call {
                    chat_request.function_call = Some(ChatCompletionFunctionCall::Function {
                        name: function_call.clone(),
                    });
                }
            }

            match client.chat().create(chat_request).await {
                Ok(response) => {
                    let responses: Vec<String> = response.choices
                        .iter()
                        .filter_map(|choice| choice.message.content.clone())
                        .collect();

                    let function_calls: Vec<Option<FunctionCall>> = response.choices
                        .iter()
                        .map(|choice| choice.message.function_call.clone())
                        .collect();

                    let usage = response.usage.map(|u| CompletionUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    });

                    let batch_response = BatchResponse {
                        responses,
                        function_calls,
                        usage,
                        choices: response.choices,
                        model: response.model,
                        system_fingerprint: response.system_fingerprint,
                    };

                    let mut jobs = long_running_jobs.lock().await;
                    if let Some(job) = jobs.get(&job_id) {
                        *job.status.lock().await = BatchJobStatus::Completed(batch_response.clone());
                    }
                    completed_jobs.lock().await.push_back((job_id.clone(), batch_response));
                }
                Err(e) => {
                    error!("Error processing long-running job {}: {:?}", job_id, e);
                    let mut jobs = long_running_jobs.lock().await;
                    if let Some(job) = jobs.get(&job_id) {
                        *job.status.lock().await = BatchJobStatus::Failed(format!("{:?}", e));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn submit_long_running_request(&self, request: BatchRequest) -> Result<String> {
        let job_id = Uuid::new_v4().to_string();
        let job = BatchJob {
            id: job_id.clone(),
            request,
            status: Arc::new(Mutex::new(BatchJobStatus::Queued)),
            created_at: Instant::now(),
        };

        self.long_running_jobs.lock().await.insert(job_id.clone(), job);
        Ok(job_id)
    }

    pub async fn get_job_status(&self, job_id: &str) -> Option<BatchJobStatus> {
        if let Some(job) = self.long_running_jobs.lock().await.get(job_id) {
            Some(job.status.lock().await.clone())
        } else {
            None
        }
    }

    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        if let Some(job) = self.long_running_jobs.lock().await.get(job_id) {
            *job.status.lock().await = BatchJobStatus::Cancelled;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Job not found"))
        }
    }

    pub async fn submit_request(&self, request: BatchRequest) -> Result<BatchResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.lock().await.push((request, tx));
        rx.await?
    }
}

#[async_trait]
impl ToolExecutor for GPTBatchTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let is_long_running = params.get("long_running")
            .map(|v| v.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);

        let request = BatchRequest {
            messages: vec![params.get("prompt").unwrap_or(&String::new()).clone()],
            model: params.get("model").unwrap_or(&"gpt-4".to_string()).clone(),
            temperature: params.get("temperature").and_then(|t| t.parse().ok()),
            max_tokens: params.get("max_tokens").and_then(|t| t.parse().ok()),
            functions: None,
            function_call: None,
            is_long_running,
        };

        if is_long_running {
            let job_id = self.submit_long_running_request(request).await?;
            Ok(format!("Long-running job submitted with ID: {}", job_id))
        } else {
            debug!("Submitting real-time request: {:?}", request);
            let response = self.submit_request(request).await?;
            Ok(response.responses.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::timeout;
    use std::time::Duration;
    use mockall::predicate::*;
    use mockall::mock;

    // Mock the OpenAI client for testing
    mock! {
        pub OpenAIClient {
            async fn create_chat_completion(&self, request: CreateChatCompletionRequest) -> Result<CreateChatCompletionResponse>;
        }
    }

    #[tokio::test]
    async fn test_batch_tool_basic() {
        let tool = GPTBatchTool::new("test-key".to_string());
        let request = BatchRequest {
            messages: vec!["Test message".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            functions: None,
            function_call: None,
            is_long_running: false,
        };

        // Since we can't hit the API, we'll just verify the request structure
        let result = tool.submit_request(request).await;
        assert!(result.is_err()); // Expected since we're not actually connecting to OpenAI
    }

    #[tokio::test]
    async fn test_long_running_batch_job() {
        let tool = GPTBatchTool::new("test-key".to_string());
        let request = BatchRequest {
            messages: vec!["Long running test".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            functions: None,
            function_call: None,
            is_long_running: true,
        };

        // Submit job
        let job_id = tool.submit_long_running_request(request).await.unwrap();

        // Check initial status
        let status = tool.get_job_status(&job_id).await.unwrap();
        assert!(matches!(status, BatchJobStatus::Queued));

        // Since we can't hit the API, we'll just verify the job was created and can be cancelled
        tool.cancel_job(&job_id).await.unwrap();
        let status = tool.get_job_status(&job_id).await.unwrap();
        assert!(matches!(status, BatchJobStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_job_cancellation() {
        let tool = GPTBatchTool::new("test-key".to_string());
        let request = BatchRequest {
            messages: vec!["Job to cancel".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            functions: None,
            function_call: None,
            is_long_running: true,
        };

        // Submit and immediately cancel
        let job_id = tool.submit_long_running_request(request).await.unwrap();
        tool.cancel_job(&job_id).await.unwrap();

        // Verify cancelled status
        let status = tool.get_job_status(&job_id).await.unwrap();
        assert!(matches!(status, BatchJobStatus::Cancelled));
    }
}
