use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "recist.io",
    version = "v1alpha1",
    kind = "HealingEvent",
    plural = "healingevents",
    shortname = "he",
    namespaced,
    status = "HealingEventStatus",
    printcolumn = r#"{"name":"Phase","type":"string","jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Target Pod","type":"string","jsonPath":".spec.targetPod"}"#,
    printcolumn = r#"{"name":"Reason","type":"string","jsonPath":".spec.triggerReason"}"#,
    printcolumn = r#"{"name":"Success","type":"boolean","jsonPath":".status.outcome.success"}"#,
    printcolumn = r#"{"name":"Duration","type":"string","jsonPath":".status.durationMs"}"#,
    printcolumn = r#"{"name":"Age","type":"date","jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct HealingEventSpec {
    pub policy_ref: String,
    pub target_pod: String,
    pub target_namespace: String,
    pub trigger_reason: TriggerReason,
    #[serde(default)]
    pub trigger_metrics: Option<TriggerMetrics>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TriggerReason {
    HighCpu,
    HighMemory,
    HighLatency,
    HighErrorRate,
    CrashLoop,
    OomKilled,
    NetworkError,
    DependencyFailure,
    Unknown,
}

impl std::fmt::Display for TriggerReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerReason::HighCpu => write!(f, "HighCpu"),
            TriggerReason::HighMemory => write!(f, "HighMemory"),
            TriggerReason::HighLatency => write!(f, "HighLatency"),
            TriggerReason::HighErrorRate => write!(f, "HighErrorRate"),
            TriggerReason::CrashLoop => write!(f, "CrashLoop"),
            TriggerReason::OomKilled => write!(f, "OomKilled"),
            TriggerReason::NetworkError => write!(f, "NetworkError"),
            TriggerReason::DependencyFailure => write!(f, "DependencyFailure"),
            TriggerReason::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerMetrics {
    #[serde(default)]
    pub cpu_usage: Option<f64>,
    #[serde(default)]
    pub memory_usage: Option<f64>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub error_rate: Option<f64>,
    #[serde(default)]
    pub restart_count: Option<i32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealingEventStatus {
    pub phase: HealingPhase,

    #[serde(default)]
    pub start_time: Option<String>,

    #[serde(default)]
    pub end_time: Option<String>,

    #[serde(default)]
    pub duration_ms: Option<i64>,

    #[serde(default)]
    pub diagnosis: Option<DiagnosisResult>,

    #[serde(default)]
    pub applied_actions: Vec<AppliedAction>,

    #[serde(default)]
    pub outcome: Option<HealingOutcome>,

    #[serde(default)]
    pub causal_graph: Option<CausalGraph>,

    #[serde(default)]
    pub knowledge_entry_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum HealingPhase {
    #[default]
    Pending,
    Containing,
    Diagnosing,
    Healing,
    Verifying,
    Completed,
    Failed,
}

impl std::fmt::Display for HealingPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealingPhase::Pending => write!(f, "Pending"),
            HealingPhase::Containing => write!(f, "Containing"),
            HealingPhase::Diagnosing => write!(f, "Diagnosing"),
            HealingPhase::Healing => write!(f, "Healing"),
            HealingPhase::Verifying => write!(f, "Verifying"),
            HealingPhase::Completed => write!(f, "Completed"),
            HealingPhase::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisResult {
    pub hypothesis: String,
    pub confidence: f64,
    pub root_cause: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub related_logs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppliedAction {
    pub action_type: ActionType,
    pub timestamp: String,
    pub result: ActionResult,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub rollback_info: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    PodRestart,
    HorizontalScale,
    VerticalScale,
    ConfigUpdate,
    NetworkIsolation,
    NetworkRestore,
    DependencyRestart,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::PodRestart => write!(f, "PodRestart"),
            ActionType::HorizontalScale => write!(f, "HorizontalScale"),
            ActionType::VerticalScale => write!(f, "VerticalScale"),
            ActionType::ConfigUpdate => write!(f, "ConfigUpdate"),
            ActionType::NetworkIsolation => write!(f, "NetworkIsolation"),
            ActionType::NetworkRestore => write!(f, "NetworkRestore"),
            ActionType::DependencyRestart => write!(f, "DependencyRestart"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionResult {
    Success,
    Failed,
    Pending,
    RolledBack,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealingOutcome {
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub verification_method: Option<String>,
    #[serde(default)]
    pub metrics_after: Option<TriggerMetrics>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CausalGraph {
    pub nodes: Vec<CausalNode>,
    pub edges: Vec<CausalEdge>,
    #[serde(default)]
    pub root_cause_node_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CausalNode {
    pub id: String,
    pub node_type: CausalNodeType,
    pub description: String,
    pub timestamp: String,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CausalNodeType {
    Error,
    Warning,
    Symptom,
    RootCause,
    Metric,
    Event,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CausalEdge {
    pub from_node: String,
    pub to_node: String,
    pub relation_type: String,
    #[serde(default)]
    pub confidence: Option<f64>,
}
