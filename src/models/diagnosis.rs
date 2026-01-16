use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosisHypothesis {
    pub hypothesis: String,
    pub confidence: f64,
    pub root_cause: String,
    pub evidence: Vec<Evidence>,
    pub causal_tree: CausalTree,
    pub created_at: DateTime<Utc>,
}

impl DiagnosisHypothesis {
    pub fn new(hypothesis: String, confidence: f64, root_cause: String) -> Self {
        Self {
            hypothesis,
            confidence,
            root_cause,
            evidence: Vec::new(),
            causal_tree: CausalTree::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
    }

    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub source: EvidenceSource,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub relevance_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceSource {
    Log,
    Metric,
    KubernetesEvent,
    TraceSpan,
    PreviousIncident,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CausalTree {
    pub nodes: HashMap<String, CausalNode>,
    pub edges: Vec<CausalEdge>,
    pub root_node_id: Option<String>,
}

impl CausalTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: CausalNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, from: String, to: String, relation: CausalRelation) {
        self.edges.push(CausalEdge {
            from_node_id: from,
            to_node_id: to,
            relation,
            confidence: None,
        });
    }

    pub fn set_root(&mut self, node_id: String) {
        self.root_node_id = Some(node_id);
    }

    pub fn get_root_cause_chain(&self) -> Vec<&CausalNode> {
        let mut chain = Vec::new();
        if let Some(root_id) = &self.root_node_id {
            self.collect_chain(root_id, &mut chain);
        }
        chain
    }

    fn collect_chain<'a>(&'a self, node_id: &str, chain: &mut Vec<&'a CausalNode>) {
        if let Some(node) = self.nodes.get(node_id) {
            chain.push(node);
            for edge in &self.edges {
                if edge.from_node_id == node_id {
                    self.collect_chain(&edge.to_node_id, chain);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalNode {
    pub id: String,
    pub node_type: CausalNodeType,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub severity: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl CausalNode {
    pub fn new(id: String, node_type: CausalNodeType, description: String, source: String) -> Self {
        Self {
            id,
            node_type,
            description,
            timestamp: Utc::now(),
            source,
            severity: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalNodeType {
    Error,
    Warning,
    Info,
    Symptom,
    RootCause,
    Metric,
    Event,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub relation: CausalRelation,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalRelation {
    Causes,
    Precedes,
    Correlates,
    Triggers,
    DependsOn,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuredLog {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
    pub pod_name: String,
    pub namespace: String,
    pub container_name: Option<String>,
    pub labels: HashMap<String, String>,
    pub stack_trace: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogAnalysisRequest {
    pub logs: Vec<StructuredLog>,
    pub metrics: HashMap<String, f64>,
    pub kubernetes_events: Vec<String>,
    pub pod_name: String,
    pub namespace: String,
    pub lookback_minutes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmDiagnosisResponse {
    pub root_cause: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub explanation: String,
    pub suggested_actions: Vec<String>,
}
