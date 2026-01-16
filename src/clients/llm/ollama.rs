use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

use super::traits::*;
use crate::error::{RecistError, Result};
use crate::models::{LlmDiagnosisResponse, StrategyEvaluation, StrategyType};

pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| RecistError::LlmError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        })
    }

    async fn send_request(&self, system: Option<&str>, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.1),
                num_predict: Some(4096),
            }),
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecistError::LlmError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LlmError(format!(
                "Ollama API error {}: {}",
                status, body
            )));
        }

        let result: OllamaResponse = response
            .json()
            .await
            .map_err(|e| RecistError::LlmError(format!("Failed to parse response: {}", e)))?;

        Ok(result.response)
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.send_request(None, prompt).await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        self.send_request(Some(system), prompt).await
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
        let url = format!("{}/api/embeddings", self.base_url);

        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecistError::LlmError(format!("Embedding request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LlmError(format!(
                "Ollama embedding error {}: {}",
                status, body
            )));
        }

        let result: OllamaEmbedResponse = response.json().await.map_err(|e| {
            RecistError::LlmError(format!("Failed to parse embedding response: {}", e))
        })?;

        Ok(result.embedding)
    }

    fn provider_name(&self) -> &str {
        "Ollama"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
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
