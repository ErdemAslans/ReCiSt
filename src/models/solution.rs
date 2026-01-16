use crate::crd::ActionType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionStrategy {
    pub strategy_type: StrategyType,
    pub actions: Vec<PlannedAction>,
    pub confidence: f64,
    pub risk_level: RiskLevel,
    pub estimated_duration_seconds: u64,
    pub rollback_plan: Option<RollbackPlan>,
    pub selected_at: DateTime<Utc>,
}

impl SolutionStrategy {
    pub fn new(strategy_type: StrategyType, confidence: f64) -> Self {
        let risk_level = strategy_type.default_risk_level();
        let estimated_duration = strategy_type.estimated_duration_seconds();

        Self {
            strategy_type,
            actions: Vec::new(),
            confidence,
            risk_level,
            estimated_duration_seconds: estimated_duration,
            rollback_plan: None,
            selected_at: Utc::now(),
        }
    }

    pub fn add_action(&mut self, action: PlannedAction) {
        self.actions.push(action);
    }

    pub fn set_rollback_plan(&mut self, plan: RollbackPlan) {
        self.rollback_plan = Some(plan);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyType {
    PodRestart,
    HorizontalScale,
    VerticalScale,
    ConfigUpdate,
    DependencyRestart,
    NetworkIsolation,
    Composite,
}

impl StrategyType {
    pub fn default_risk_level(&self) -> RiskLevel {
        match self {
            StrategyType::PodRestart => RiskLevel::Low,
            StrategyType::HorizontalScale => RiskLevel::Low,
            StrategyType::VerticalScale => RiskLevel::Medium,
            StrategyType::ConfigUpdate => RiskLevel::Medium,
            StrategyType::DependencyRestart => RiskLevel::High,
            StrategyType::NetworkIsolation => RiskLevel::Low,
            StrategyType::Composite => RiskLevel::Medium,
        }
    }

    pub fn estimated_duration_seconds(&self) -> u64 {
        match self {
            StrategyType::PodRestart => 30,
            StrategyType::HorizontalScale => 60,
            StrategyType::VerticalScale => 120,
            StrategyType::ConfigUpdate => 60,
            StrategyType::DependencyRestart => 120,
            StrategyType::NetworkIsolation => 10,
            StrategyType::Composite => 180,
        }
    }

    pub fn to_action_type(&self) -> ActionType {
        match self {
            StrategyType::PodRestart => ActionType::PodRestart,
            StrategyType::HorizontalScale => ActionType::HorizontalScale,
            StrategyType::VerticalScale => ActionType::VerticalScale,
            StrategyType::ConfigUpdate => ActionType::ConfigUpdate,
            StrategyType::DependencyRestart => ActionType::DependencyRestart,
            StrategyType::NetworkIsolation => ActionType::NetworkIsolation,
            StrategyType::Composite => ActionType::PodRestart,
        }
    }
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::PodRestart => write!(f, "PodRestart"),
            StrategyType::HorizontalScale => write!(f, "HorizontalScale"),
            StrategyType::VerticalScale => write!(f, "VerticalScale"),
            StrategyType::ConfigUpdate => write!(f, "ConfigUpdate"),
            StrategyType::DependencyRestart => write!(f, "DependencyRestart"),
            StrategyType::NetworkIsolation => write!(f, "NetworkIsolation"),
            StrategyType::Composite => write!(f, "Composite"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedAction {
    pub action_type: ActionType,
    pub target: ActionTarget,
    pub parameters: HashMap<String, String>,
    pub order: u32,
    pub depends_on: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionTarget {
    pub resource_type: ResourceType,
    pub name: String,
    pub namespace: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Pod,
    Deployment,
    StatefulSet,
    DaemonSet,
    ConfigMap,
    Secret,
    Service,
    NetworkPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub actions: Vec<RollbackAction>,
    pub timeout_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackAction {
    pub action_type: RollbackActionType,
    pub target: ActionTarget,
    pub original_state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RollbackActionType {
    RestoreReplicas,
    RestoreResources,
    RestoreConfig,
    DeleteNetworkPolicy,
    RestartPod,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicroAgentResult {
    pub agent_id: String,
    pub hypothesis: String,
    pub strategy_type: StrategyType,
    pub confidence: f64,
    pub reasoning_depth: u32,
    pub evidence: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyEvaluation {
    pub strategy_type: StrategyType,
    pub success_probability: f64,
    pub risk_score: f64,
    pub estimated_time_seconds: u64,
    pub reasoning: String,
    pub prerequisites_met: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_type: ActionType,
    pub success: bool,
    pub message: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub rollback_data: Option<String>,
}
