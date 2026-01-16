use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

use super::traits::*;
use crate::error::{RecistError, Result};
use crate::models::{LlmDiagnosisResponse, StrategyEvaluation, StrategyType};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
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

    async fn send_request(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_URL, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.1),
                max_output_tokens: Some(4096),
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
                "Gemini API error {}: {}",
                status, body
            )));
        }

        let result: GeminiResponse = response
            .json()
            .await
            .map_err(|e| RecistError::LlmError(format!("Failed to parse response: {}", e)))?;

        let text = result
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .unwrap_or_default();

        Ok(text)
    }
}

#[async_trait]
impl LlmClient for GeminiClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.send_request(prompt).await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        let full_prompt = format!("{}\n\n{}", system, prompt);
        self.send_request(&full_prompt).await
    }

    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<LlmDiagnosisResponse> {
        let prompt = build_diagnosis_prompt(request);
        let full_prompt = format!("{}\n\n{}", DIAGNOSIS_SYSTEM_PROMPT, prompt);
        let response = self.send_request(&full_prompt).await?;

        parse_diagnosis_response(&response)
    }

    async fn evaluate_strategy(
        &self,
        request: &StrategyEvaluationRequest,
    ) -> Result<StrategyEvaluation> {
        let prompt = build_strategy_evaluation_prompt(request);
        let full_prompt = format!("{}\n\n{}", STRATEGY_EVALUATION_SYSTEM_PROMPT, prompt);
        let response = self.send_request(&full_prompt).await?;

        parse_strategy_evaluation(&response, &request.strategy_type)
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "{}/embedding-001:embedContent?key={}",
            GEMINI_API_URL, self.api_key
        );

        let request = EmbedRequest {
            model: "models/embedding-001".to_string(),
            content: EmbedContent {
                parts: vec![EmbedPart {
                    text: text.to_string(),
                }],
            },
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
                "Gemini embedding error {}: {}",
                status, body
            )));
        }

        let result: EmbedResponse = response.json().await.map_err(|e| {
            RecistError::LlmError(format!("Failed to parse embedding response: {}", e))
        })?;

        Ok(result.embedding.values)
    }

    fn provider_name(&self) -> &str {
        "Gemini"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    content: EmbedContent,
}

#[derive(Debug, Serialize)]
struct EmbedContent {
    parts: Vec<EmbedPart>,
}

#[derive(Debug, Serialize)]
struct EmbedPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: EmbedValues,
}

#[derive(Debug, Deserialize)]
struct EmbedValues {
    values: Vec<f32>,
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
