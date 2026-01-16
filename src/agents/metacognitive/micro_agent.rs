use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::clients::llm::{LlmClient, MetricSnapshot, StrategyEvaluationRequest};
use crate::error::Result;
use crate::models::{DiagnosisHypothesis, MicroAgentResult, StrategyType};

pub struct MicroAgent {
    id: String,
    strategy_type: StrategyType,
    hypothesis: DiagnosisHypothesis,
    llm: Arc<dyn LlmClient>,
    max_depth: u32,
}

impl MicroAgent {
    pub fn new(
        strategy_type: StrategyType,
        hypothesis: DiagnosisHypothesis,
        llm: Arc<dyn LlmClient>,
        max_depth: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            strategy_type,
            hypothesis,
            llm,
            max_depth,
        }
    }

    pub async fn evaluate(self) -> Result<MicroAgentResult> {
        debug!(
            "Micro-agent {} evaluating strategy {:?}",
            self.id, self.strategy_type
        );

        let mut confidence = self.calculate_initial_confidence();
        let mut evidence = Vec::new();
        let mut depth = 0;

        while confidence < 0.8 && depth < self.max_depth {
            let request = StrategyEvaluationRequest {
                diagnosis: self.hypothesis.hypothesis.clone(),
                root_cause: self.hypothesis.root_cause.clone(),
                strategy_type: self.strategy_type.to_string(),
                current_metrics: vec![MetricSnapshot {
                    name: "confidence".to_string(),
                    value: confidence,
                    threshold: Some(0.8),
                }],
                historical_success_rate: Some(self.get_historical_success_rate()),
            };

            let evaluation = self.llm.evaluate_strategy(&request).await?;

            confidence = evaluation.success_probability;
            evidence.push(evaluation.reasoning);

            depth += 1;

            if evaluation.prerequisites_met && confidence >= 0.7 {
                break;
            }
        }

        info!(
            "Micro-agent {} completed: strategy={:?}, confidence={:.2}, depth={}",
            self.id, self.strategy_type, confidence, depth
        );

        Ok(MicroAgentResult {
            agent_id: self.id,
            hypothesis: self.hypothesis.hypothesis.clone(),
            strategy_type: self.strategy_type,
            confidence,
            reasoning_depth: depth,
            evidence,
            completed_at: Utc::now(),
        })
    }

    fn calculate_initial_confidence(&self) -> f64 {
        let root_cause_lower = self.hypothesis.root_cause.to_lowercase();

        match &self.strategy_type {
            StrategyType::PodRestart => {
                if root_cause_lower.contains("memory") || root_cause_lower.contains("leak") {
                    0.7
                } else if root_cause_lower.contains("deadlock") || root_cause_lower.contains("hang")
                {
                    0.65
                } else {
                    0.5
                }
            }
            StrategyType::HorizontalScale => {
                if root_cause_lower.contains("load") || root_cause_lower.contains("capacity") {
                    0.7
                } else if root_cause_lower.contains("cpu") {
                    0.6
                } else {
                    0.4
                }
            }
            StrategyType::VerticalScale => {
                if root_cause_lower.contains("oom") || root_cause_lower.contains("memory") {
                    0.65
                } else if root_cause_lower.contains("cpu") {
                    0.55
                } else {
                    0.4
                }
            }
            StrategyType::ConfigUpdate => {
                if root_cause_lower.contains("connection") || root_cause_lower.contains("pool") {
                    0.6
                } else if root_cause_lower.contains("timeout") {
                    0.55
                } else {
                    0.35
                }
            }
            StrategyType::DependencyRestart => {
                if root_cause_lower.contains("dependency") || root_cause_lower.contains("upstream")
                {
                    0.5
                } else {
                    0.3
                }
            }
            StrategyType::NetworkIsolation => {
                if root_cause_lower.contains("network") || root_cause_lower.contains("cascade") {
                    0.6
                } else {
                    0.35
                }
            }
            StrategyType::Composite => 0.5,
        }
    }

    fn get_historical_success_rate(&self) -> f64 {
        match &self.strategy_type {
            StrategyType::PodRestart => 0.85,
            StrategyType::HorizontalScale => 0.75,
            StrategyType::VerticalScale => 0.70,
            StrategyType::ConfigUpdate => 0.65,
            StrategyType::DependencyRestart => 0.60,
            StrategyType::NetworkIsolation => 0.80,
            StrategyType::Composite => 0.70,
        }
    }
}
