use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "recist.io",
    version = "v1alpha1",
    kind = "SelfHealingPolicy",
    plural = "selfhealingpolicies",
    shortname = "shp",
    namespaced,
    status = "SelfHealingPolicyStatus",
    printcolumn = r#"{"name":"Active Healings","type":"integer","jsonPath":".status.activeHealings"}"#,
    printcolumn = r#"{"name":"Last Healing","type":"date","jsonPath":".status.lastHealingTime"}"#,
    printcolumn = r#"{"name":"Age","type":"date","jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct SelfHealingPolicySpec {
    #[serde(default)]
    pub target_namespaces: Vec<String>,

    #[serde(default)]
    pub target_labels: BTreeMap<String, String>,

    pub thresholds: Thresholds,

    #[serde(default)]
    pub allowed_actions: Vec<AllowedAction>,

    pub llm_config: LlmConfig,

    #[serde(default)]
    pub notifications: Option<NotificationConfig>,

    #[serde(default)]
    pub containment_config: ContainmentConfig,

    #[serde(default)]
    pub diagnosis_config: DiagnosisConfig,

    #[serde(default)]
    pub metacognitive_config: MetaCognitiveConfig,

    #[serde(default)]
    pub knowledge_config: KnowledgeConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    #[serde(default = "default_cpu_threshold")]
    pub cpu: f64,

    #[serde(default = "default_memory_threshold")]
    pub memory: f64,

    #[serde(default = "default_latency_threshold")]
    pub latency_ms: u64,

    #[serde(default = "default_error_rate_threshold")]
    pub error_rate: f64,
}

fn default_cpu_threshold() -> f64 {
    0.9
}
fn default_memory_threshold() -> f64 {
    0.85
}
fn default_latency_threshold() -> u64 {
    500
}
fn default_error_rate_threshold() -> f64 {
    0.05
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AllowedAction {
    Restart,
    Scale,
    UpdateConfig,
    UpdateResources,
    Isolate,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub provider: LlmProvider,

    pub model: String,

    pub api_key_secret: String,

    #[serde(default = "default_llm_timeout")]
    pub timeout_seconds: u64,

    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_llm_timeout() -> u64 {
    30
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Claude,
    OpenAI,
    Gemini,
    Ollama,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub slack_webhook: Option<String>,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub pagerduty_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainmentConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,

    #[serde(default = "default_isolation_strategy")]
    pub isolation_strategy: IsolationStrategy,

    #[serde(default = "default_neighbor_capacity_threshold")]
    pub neighbor_capacity_threshold: f64,
}

impl Default for ContainmentConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval(),
            isolation_strategy: default_isolation_strategy(),
            neighbor_capacity_threshold: default_neighbor_capacity_threshold(),
        }
    }
}

fn default_check_interval() -> u64 {
    10
}
fn default_isolation_strategy() -> IsolationStrategy {
    IsolationStrategy::Soft
}
fn default_neighbor_capacity_threshold() -> f64 {
    0.7
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IsolationStrategy {
    Soft,
    Hard,
    Auto,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisConfig {
    #[serde(default = "default_log_lookback")]
    pub log_lookback_minutes: u64,

    #[serde(default = "default_max_log_lines")]
    pub max_log_lines: u64,

    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
}

impl Default for DiagnosisConfig {
    fn default() -> Self {
        Self {
            log_lookback_minutes: default_log_lookback(),
            max_log_lines: default_max_log_lines(),
            confidence_threshold: default_confidence_threshold(),
        }
    }
}

fn default_log_lookback() -> u64 {
    5
}
fn default_max_log_lines() -> u64 {
    1000
}
fn default_confidence_threshold() -> f64 {
    0.7
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetaCognitiveConfig {
    #[serde(default = "default_max_micro_agents")]
    pub max_micro_agents: u32,

    #[serde(default = "default_max_reasoning_depth")]
    pub max_reasoning_depth: u32,

    #[serde(default = "default_action_timeout")]
    pub action_timeout_seconds: u64,

    #[serde(default = "default_verification_wait")]
    pub verification_wait_seconds: u64,

    #[serde(default = "default_decision_threshold")]
    pub decision_threshold: f64,
}

impl Default for MetaCognitiveConfig {
    fn default() -> Self {
        Self {
            max_micro_agents: default_max_micro_agents(),
            max_reasoning_depth: default_max_reasoning_depth(),
            action_timeout_seconds: default_action_timeout(),
            verification_wait_seconds: default_verification_wait(),
            decision_threshold: default_decision_threshold(),
        }
    }
}

fn default_max_micro_agents() -> u32 {
    5
}
fn default_max_reasoning_depth() -> u32 {
    10
}
fn default_action_timeout() -> u64 {
    60
}
fn default_verification_wait() -> u64 {
    30
}
fn default_decision_threshold() -> f64 {
    0.7
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeConfig {
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,

    #[serde(default = "default_max_local_events")]
    pub max_local_events: u64,

    #[serde(default = "default_knowledge_ttl")]
    pub knowledge_ttl_days: u64,

    #[serde(default = "default_embedding_dimensions")]
    pub embedding_dimensions: u32,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: default_similarity_threshold(),
            max_local_events: default_max_local_events(),
            knowledge_ttl_days: default_knowledge_ttl(),
            embedding_dimensions: default_embedding_dimensions(),
        }
    }
}

fn default_similarity_threshold() -> f64 {
    0.8
}
fn default_max_local_events() -> u64 {
    100
}
fn default_knowledge_ttl() -> u64 {
    90
}
fn default_embedding_dimensions() -> u32 {
    1536
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelfHealingPolicyStatus {
    #[serde(default)]
    pub observed_generation: i64,

    #[serde(default)]
    pub active_healings: i32,

    #[serde(default)]
    pub last_healing_time: Option<String>,

    #[serde(default)]
    pub total_healings: i64,

    #[serde(default)]
    pub successful_healings: i64,

    #[serde(default)]
    pub conditions: Vec<PolicyCondition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PolicyCondition {
    pub condition_type: String,
    pub status: String,
    pub last_transition_time: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
