use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

use super::traits::*;
use crate::error::{RecistError, Result};
use crate::models::{LlmDiagnosisResponse, StrategyEvaluation, StrategyType};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
}

impl ClaudeClient {
    pub fn new(api_key: &str, model: &str, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| RecistError::LlmError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }

    async fn send_request(&self, system: Option<&str>, messages: Vec<Message>) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system.map(|s| s.to_string()),
            messages,
        };

        let response = self
            .client
            .post(CLAUDE_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecistError::LlmError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LlmError(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let result: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| RecistError::LlmError(format!("Failed to parse response: {}", e)))?;

        let text = result
            .content
            .into_iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    Some(c.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }
}

#[async_trait]
impl LlmClient for ClaudeClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.send_request(None, messages).await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.send_request(Some(system), messages).await
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
        Err(RecistError::LlmError(
            "Claude does not support embeddings directly. Use a separate embedding model."
                .to_string(),
        ))
    }

    fn provider_name(&self) -> &str {
        "Claude"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
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

fn parse_strategy_evaluation(response: &str, strategy_type: &str) -> Result<StrategyEvaluation> {
    let json_str = extract_json(response);

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| RecistError::LlmError(format!("Failed to parse evaluation JSON: {}", e)))?;

    let strategy = match strategy_type.to_lowercase().as_str() {
        "podrestart" | "pod_restart" => StrategyType::PodRestart,
        "horizontalscale" | "horizontal_scale" => StrategyType::HorizontalScale,
        "verticalscale" | "vertical_scale" => StrategyType::VerticalScale,
        "configupdate" | "config_update" => StrategyType::ConfigUpdate,
        "dependencyrestart" | "dependency_restart" => StrategyType::DependencyRestart,
        "networkisolation" | "network_isolation" => StrategyType::NetworkIsolation,
        _ => StrategyType::PodRestart,
    };

    Ok(StrategyEvaluation {
        strategy_type: strategy,
        success_probability: parsed["success_probability"].as_f64().unwrap_or(0.0),
        risk_score: parsed["risk_score"].as_f64().unwrap_or(0.5),
        estimated_time_seconds: parsed["estimated_time_seconds"].as_u64().unwrap_or(30),
        reasoning: parsed["reasoning"].as_str().unwrap_or("").to_string(),
        prerequisites_met: parsed["prerequisites_met"].as_bool().unwrap_or(true),
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
