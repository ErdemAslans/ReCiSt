use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{DiagnosisHypothesis, LlmDiagnosisResponse, StrategyEvaluation};

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String>;

    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<LlmDiagnosisResponse>;

    async fn evaluate_strategy(
        &self,
        request: &StrategyEvaluationRequest,
    ) -> Result<StrategyEvaluation>;

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;

    fn provider_name(&self) -> &str;

    fn model_name(&self) -> &str;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosisRequest {
    pub logs: Vec<String>,
    pub metrics: Vec<MetricSnapshot>,
    pub kubernetes_events: Vec<String>,
    pub pod_name: String,
    pub namespace: String,
    pub error_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub name: String,
    pub value: f64,
    pub threshold: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyEvaluationRequest {
    pub diagnosis: String,
    pub root_cause: String,
    pub strategy_type: String,
    pub current_metrics: Vec<MetricSnapshot>,
    pub historical_success_rate: Option<f64>,
}

pub const DIAGNOSIS_SYSTEM_PROMPT: &str = r#"You are an expert Site Reliability Engineer (SRE) analyzing system failures. Your task is to:

1. Analyze the provided logs, metrics, and Kubernetes events
2. Identify the root cause of the issue
3. Provide a confidence score (0-100) for your diagnosis
4. List supporting evidence from the logs

Respond in JSON format:
{
    "root_cause": "Brief description of the root cause",
    "confidence": 85,
    "evidence": ["Evidence line 1", "Evidence line 2"],
    "explanation": "Detailed explanation of the diagnosis",
    "suggested_actions": ["Action 1", "Action 2"]
}"#;

pub const STRATEGY_EVALUATION_SYSTEM_PROMPT: &str = r#"You are an expert Site Reliability Engineer evaluating healing strategies. Your task is to:

1. Evaluate if the proposed strategy is appropriate for the diagnosed issue
2. Estimate success probability based on the evidence
3. Identify any risks or prerequisites
4. Provide a risk score (0-100)

Respond in JSON format:
{
    "success_probability": 0.85,
    "risk_score": 0.2,
    "estimated_time_seconds": 30,
    "reasoning": "Why this strategy is appropriate",
    "prerequisites_met": true
}"#;

pub fn build_diagnosis_prompt(request: &DiagnosisRequest) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "Analyze the following issue for pod '{}' in namespace '{}'.\n\n",
        request.pod_name, request.namespace
    ));

    prompt.push_str(&format!("Error Type: {}\n\n", request.error_type));

    prompt.push_str("=== LOGS ===\n");
    for (i, log) in request.logs.iter().take(50).enumerate() {
        prompt.push_str(&format!("[{}] {}\n", i + 1, log));
    }
    prompt.push('\n');

    prompt.push_str("=== METRICS ===\n");
    for metric in &request.metrics {
        let threshold_str = metric
            .threshold
            .map(|t| format!(" (threshold: {})", t))
            .unwrap_or_default();
        prompt.push_str(&format!(
            "{}: {}{}\n",
            metric.name, metric.value, threshold_str
        ));
    }
    prompt.push('\n');

    prompt.push_str("=== KUBERNETES EVENTS ===\n");
    for event in &request.kubernetes_events {
        prompt.push_str(&format!("- {}\n", event));
    }
    prompt.push('\n');

    prompt.push_str("Based on the above information, provide your diagnosis in JSON format.");

    prompt
}

pub fn build_strategy_evaluation_prompt(request: &StrategyEvaluationRequest) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "Evaluate the '{}' strategy for the following issue:\n\n",
        request.strategy_type
    ));

    prompt.push_str(&format!("Diagnosis: {}\n", request.diagnosis));
    prompt.push_str(&format!("Root Cause: {}\n\n", request.root_cause));

    prompt.push_str("=== CURRENT METRICS ===\n");
    for metric in &request.current_metrics {
        prompt.push_str(&format!("{}: {}\n", metric.name, metric.value));
    }
    prompt.push('\n');

    if let Some(rate) = request.historical_success_rate {
        prompt.push_str(&format!(
            "Historical success rate for this strategy: {:.1}%\n\n",
            rate * 100.0
        ));
    }

    prompt.push_str(
        "Evaluate if this strategy is appropriate and provide your assessment in JSON format.",
    );

    prompt
}
