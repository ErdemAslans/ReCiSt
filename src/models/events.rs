use super::{DiagnosisHypothesis, FaultCluster, KnowledgeEntry, SolutionStrategy};
use crate::crd::{TriggerMetrics, TriggerReason};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: Uuid,
    pub event_type: AgentEventType,
    pub timestamp: DateTime<Utc>,
    pub source_agent: AgentType,
    pub correlation_id: Uuid,
    pub payload: EventPayload,
}

impl AgentEvent {
    pub fn new(
        event_type: AgentEventType,
        source_agent: AgentType,
        correlation_id: Uuid,
        payload: EventPayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            source_agent,
            correlation_id,
            payload,
        }
    }

    pub fn fault_detected(correlation_id: Uuid, fault_cluster: FaultCluster) -> Self {
        Self::new(
            AgentEventType::FaultDetected,
            AgentType::Containment,
            correlation_id,
            EventPayload::FaultDetected(FaultDetectedPayload { fault_cluster }),
        )
    }

    pub fn containment_complete(
        correlation_id: Uuid,
        pod_name: String,
        namespace: String,
        isolated: bool,
    ) -> Self {
        Self::new(
            AgentEventType::ContainmentComplete,
            AgentType::Containment,
            correlation_id,
            EventPayload::ContainmentComplete(ContainmentCompletePayload {
                pod_name,
                namespace,
                isolated,
                isolation_method: if isolated {
                    Some("NetworkPolicy".to_string())
                } else {
                    None
                },
            }),
        )
    }

    pub fn diagnosis_complete(correlation_id: Uuid, hypothesis: DiagnosisHypothesis) -> Self {
        Self::new(
            AgentEventType::DiagnosisComplete,
            AgentType::Diagnosis,
            correlation_id,
            EventPayload::DiagnosisComplete(DiagnosisCompletePayload { hypothesis }),
        )
    }

    pub fn healing_complete(
        correlation_id: Uuid,
        strategy: SolutionStrategy,
        success: bool,
        message: String,
    ) -> Self {
        Self::new(
            AgentEventType::HealingComplete,
            AgentType::MetaCognitive,
            correlation_id,
            EventPayload::HealingComplete(HealingCompletePayload {
                strategy,
                success,
                message,
            }),
        )
    }

    pub fn knowledge_updated(correlation_id: Uuid, entry: KnowledgeEntry) -> Self {
        Self::new(
            AgentEventType::KnowledgeUpdated,
            AgentType::Knowledge,
            correlation_id,
            EventPayload::KnowledgeUpdated(KnowledgeUpdatedPayload { entry }),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentEventType {
    FaultDetected,
    ContainmentComplete,
    DiagnosisComplete,
    HealingComplete,
    KnowledgeUpdated,
    ProactiveWarning,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    Containment,
    Diagnosis,
    MetaCognitive,
    Knowledge,
    Controller,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Containment => write!(f, "Containment"),
            AgentType::Diagnosis => write!(f, "Diagnosis"),
            AgentType::MetaCognitive => write!(f, "MetaCognitive"),
            AgentType::Knowledge => write!(f, "Knowledge"),
            AgentType::Controller => write!(f, "Controller"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventPayload {
    FaultDetected(FaultDetectedPayload),
    ContainmentComplete(ContainmentCompletePayload),
    DiagnosisComplete(DiagnosisCompletePayload),
    HealingComplete(HealingCompletePayload),
    KnowledgeUpdated(KnowledgeUpdatedPayload),
    ProactiveWarning(ProactiveWarningPayload),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaultDetectedPayload {
    pub fault_cluster: FaultCluster,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainmentCompletePayload {
    pub pod_name: String,
    pub namespace: String,
    pub isolated: bool,
    pub isolation_method: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosisCompletePayload {
    pub hypothesis: DiagnosisHypothesis,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealingCompletePayload {
    pub strategy: SolutionStrategy,
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeUpdatedPayload {
    pub entry: KnowledgeEntry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProactiveWarningPayload {
    pub namespace: String,
    pub pod_name: Option<String>,
    pub warning_type: String,
    pub message: String,
    pub suggested_action: Option<String>,
    pub confidence: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaultInfo {
    pub pod_name: String,
    pub namespace: String,
    pub reason: TriggerReason,
    pub metrics: TriggerMetrics,
    pub detected_at: DateTime<Utc>,
}
