use crate::crd::{TriggerMetrics, TriggerReason};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaultCluster {
    pub faults: Vec<Fault>,
    pub detected_at: DateTime<Utc>,
    pub namespace: String,
}

impl FaultCluster {
    pub fn new(namespace: String) -> Self {
        Self {
            faults: Vec::new(),
            detected_at: Utc::now(),
            namespace,
        }
    }

    pub fn add_fault(&mut self, fault: Fault) {
        self.faults.push(fault);
    }

    pub fn is_empty(&self) -> bool {
        self.faults.is_empty()
    }

    pub fn pod_names(&self) -> Vec<String> {
        self.faults.iter().map(|f| f.pod_name.clone()).collect()
    }

    pub fn primary_fault(&self) -> Option<&Fault> {
        self.faults.first()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fault {
    pub pod_name: String,
    pub namespace: String,
    pub reasons: Vec<TriggerReason>,
    pub metrics: TriggerMetrics,
    pub detected_at: DateTime<Utc>,
    pub severity: FaultSeverity,
    pub labels: HashMap<String, String>,
}

impl Fault {
    pub fn new(
        pod_name: String,
        namespace: String,
        reasons: Vec<TriggerReason>,
        metrics: TriggerMetrics,
    ) -> Self {
        let severity = Self::calculate_severity(&reasons, &metrics);
        Self {
            pod_name,
            namespace,
            reasons,
            metrics,
            detected_at: Utc::now(),
            severity,
            labels: HashMap::new(),
        }
    }

    fn calculate_severity(reasons: &[TriggerReason], metrics: &TriggerMetrics) -> FaultSeverity {
        let has_critical = reasons
            .iter()
            .any(|r| matches!(r, TriggerReason::OomKilled | TriggerReason::CrashLoop));

        if has_critical {
            return FaultSeverity::Critical;
        }

        let error_rate = metrics.error_rate.unwrap_or(0.0);
        if error_rate > 0.5 {
            return FaultSeverity::Critical;
        }
        if error_rate > 0.2 {
            return FaultSeverity::High;
        }

        let cpu = metrics.cpu_usage.unwrap_or(0.0);
        let memory = metrics.memory_usage.unwrap_or(0.0);
        if cpu > 0.95 || memory > 0.95 {
            return FaultSeverity::High;
        }

        FaultSeverity::Medium
    }

    pub fn primary_reason(&self) -> &TriggerReason {
        self.reasons.first().unwrap_or(&TriggerReason::Unknown)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for FaultSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultSeverity::Low => write!(f, "Low"),
            FaultSeverity::Medium => write!(f, "Medium"),
            FaultSeverity::High => write!(f, "High"),
            FaultSeverity::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolationRule {
    pub pod_name: String,
    pub namespace: String,
    pub network_policy_name: String,
    pub created_at: DateTime<Utc>,
    pub rule_type: IsolationRuleType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IsolationRuleType {
    DenyAll,
    DenyIngress,
    DenyEgress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficRedirect {
    pub from_pod: String,
    pub to_pods: Vec<String>,
    pub namespace: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeighborNegotiationResult {
    pub requesting_pod: String,
    pub accepting_pods: Vec<AcceptingNeighbor>,
    pub rejected_pods: Vec<RejectedNeighbor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptingNeighbor {
    pub pod_name: String,
    pub available_capacity: f64,
    pub accepted_load_fraction: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RejectedNeighbor {
    pub pod_name: String,
    pub reason: String,
}
