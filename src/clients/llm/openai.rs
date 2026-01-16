use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

use super::claude::{extract_json, parse_diagnosis_response, parse_strategy_evaluation};
use super::traits::*;
use crate::error::{RecistError, Result};
use crate::models::{LlmDiagnosisResponse, StrategyEvaluation};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const OPENAI_EMBEDDINGS_URL: &str = "https://api.openai.com/v1/embeddings";

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(
        api_key: &str,
        model: &str,
        base_url: Option<&str>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| RecistError::LlmError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url: base_url
                .map(|s| s.to_string())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        })
    }

    async fn send_chat_request(&self, system: Option<&str>, user_message: &str) -> Result<String> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(4096),
            temperature: Some(0.1),
        };

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecistError::LlmError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LlmError(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let result: ChatResponse = response
            .json()
            .await
            .map_err(|e| RecistError::LlmError(format!("Failed to parse response: {}", e)))?;

        let text = result
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(text)
    }
}

#[async_trait]
impl LlmClient for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.send_chat_request(None, prompt).await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        self.send_chat_request(Some(system), prompt).await
    }

    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<LlmDiagnosisResponse> {
        let prompt = build_diagnosis_prompt(request);
        let response = self
            .complete_with_system(DIAGNOSIS_SYSTEM_PROMPT, &prompt)
            .await?;

        parse_diagnosis_response(&response)
    }

    async fn evaluate_strategy(
        &self,
        request: &StrategyEvaluationRequest,
    ) -> Result<StrategyEvaluation> {
        let prompt = build_strategy_evaluation_prompt(request);
        let response = self
            .complete_with_system(STRATEGY_EVALUATION_SYSTEM_PROMPT, &prompt)
            .await?;

        parse_strategy_evaluation(&response, &request.strategy_type)
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: text.to_string(),
        };

        let url = format!("{}/embeddings", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecistError::LlmError(format!("Embedding request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LlmError(format!(
                "OpenAI embedding error {}: {}",
                status, body
            )));
        }

        let result: EmbeddingResponse = response.json().await.map_err(|e| {
            RecistError::LlmError(format!("Failed to parse embedding response: {}", e))
        })?;

        Ok(result
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .unwrap_or_default())
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

fn parse_diagnosis_response(response: &str) -> Result<LlmDiagnosisResponse> {
    let json_str = extract_json(response);

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| RecistError::LlmError(format!("Failed to parse diagnosis JSON: {}", e)))?;

    Ok(LlmDiagnosisResponse {
        root_cause: parsed["root_cause"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.0) / 100.0,
        evidence: parsed["evidence"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        explanation: parsed["explanation"].as_str().unwrap_or("").to_string(),
        suggested_actions: parsed["suggested_actions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn extract_json(text: &str) -> String {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}
