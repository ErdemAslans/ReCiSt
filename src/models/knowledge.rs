use super::{DiagnosisHypothesis, SolutionStrategy};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Uuid,
    pub namespace: String,
    pub pod_name: String,
    pub error_type: String,
    pub diagnosis: DiagnosisSummary,
    pub solution: SolutionSummary,
    pub outcome: OutcomeSummary,
    pub embedding: Option<Vec<f32>>,
    pub topic: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub success_rate: f64,
}

impl KnowledgeEntry {
    pub fn new(
        namespace: String,
        pod_name: String,
        error_type: String,
        diagnosis: DiagnosisSummary,
        solution: SolutionSummary,
        outcome: OutcomeSummary,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            namespace,
            pod_name,
            error_type,
            diagnosis,
            solution,
            outcome,
            embedding: None,
            topic: None,
            created_at: Utc::now(),
            expires_at: None,
            usage_count: 1,
            success_rate: if outcome.success { 1.0 } else { 0.0 },
        }
    }

    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
    }

    pub fn set_topic(&mut self, topic: String) {
        self.topic = Some(topic);
    }

    pub fn set_ttl_days(&mut self, days: u64) {
        self.expires_at = Some(self.created_at + chrono::Duration::days(days as i64));
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn record_usage(&mut self, success: bool) {
        self.usage_count += 1;
        let total_successes =
            (self.success_rate * (self.usage_count - 1) as f64) + if success { 1.0 } else { 0.0 };
        self.success_rate = total_successes / self.usage_count as f64;
    }

    pub fn summary_text(&self) -> String {
        format!(
            "Error: {} | Root Cause: {} | Solution: {} | Success: {}",
            self.error_type,
            self.diagnosis.root_cause,
            self.solution.strategy_type,
            self.outcome.success
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosisSummary {
    pub hypothesis: String,
    pub confidence: f64,
    pub root_cause: String,
    pub key_evidence: Vec<String>,
}

impl From<&DiagnosisHypothesis> for DiagnosisSummary {
    fn from(h: &DiagnosisHypothesis) -> Self {
        Self {
            hypothesis: h.hypothesis.clone(),
            confidence: h.confidence,
            root_cause: h.root_cause.clone(),
            key_evidence: h.evidence.iter().map(|e| e.content.clone()).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionSummary {
    pub strategy_type: String,
    pub actions: Vec<String>,
    pub duration_ms: i64,
}

impl From<&SolutionStrategy> for SolutionSummary {
    fn from(s: &SolutionStrategy) -> Self {
        Self {
            strategy_type: s.strategy_type.to_string(),
            actions: s
                .actions
                .iter()
                .map(|a| a.action_type.to_string())
                .collect(),
            duration_ms: s.estimated_duration_seconds as i64 * 1000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutcomeSummary {
    pub success: bool,
    pub message: String,
    pub total_duration_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub centroid: Option<Vec<f32>>,
    pub entry_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Topic {
    pub fn new(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            centroid: None,
            entry_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_centroid(&mut self, embeddings: &[Vec<f32>]) {
        if embeddings.is_empty() {
            return;
        }

        let dim = embeddings[0].len();
        let mut centroid = vec![0.0f32; dim];

        for embedding in embeddings {
            for (i, val) in embedding.iter().enumerate() {
                centroid[i] += val;
            }
        }

        let count = embeddings.len() as f32;
        for val in &mut centroid {
            *val /= count;
        }

        self.centroid = Some(centroid);
        self.entry_count = embeddings.len() as u64;
        self.updated_at = Utc::now();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilaritySearchResult {
    pub entry: KnowledgeEntry,
    pub similarity_score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProactivePrediction {
    pub namespace: String,
    pub pod_name: Option<String>,
    pub predicted_error_type: String,
    pub probability: f64,
    pub time_horizon_minutes: u64,
    pub suggested_action: Option<String>,
    pub based_on_entries: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub namespace: String,
    pub metric_name: String,
    pub current_value: f64,
    pub trend_direction: TrendDirection,
    pub change_rate_per_minute: f64,
    pub predicted_threshold_breach_minutes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}
