use crate::crd::HealingPhase;
use crate::error::{RecistError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealingState {
    pub id: Uuid,
    pub phase: HealingPhase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub transitions: Vec<StateTransition>,
}

impl HealingState {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            phase: HealingPhase::Pending,
            created_at: now,
            updated_at: now,
            transitions: vec![StateTransition {
                from: None,
                to: HealingPhase::Pending,
                timestamp: now,
                reason: Some("Initial state".to_string()),
            }],
        }
    }

    pub fn transition_to(&mut self, target: HealingPhase, reason: Option<String>) -> Result<()> {
        if !self.is_valid_transition(&target) {
            return Err(RecistError::InvalidStateTransition {
                from: self.phase.to_string(),
                to: target.to_string(),
            });
        }

        let now = Utc::now();
        self.transitions.push(StateTransition {
            from: Some(self.phase.clone()),
            to: target.clone(),
            timestamp: now,
            reason,
        });
        self.phase = target;
        self.updated_at = now;
        Ok(())
    }

    fn is_valid_transition(&self, target: &HealingPhase) -> bool {
        use HealingPhase::*;

        match (&self.phase, target) {
            (Pending, Containing) => true,
            (Containing, Diagnosing) => true,
            (Diagnosing, Healing) => true,
            (Healing, Verifying) => true,
            (Verifying, Completed) => true,
            (Verifying, Failed) => true,
            (_, Failed) => true,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.phase, HealingPhase::Completed | HealingPhase::Failed)
    }

    pub fn duration_ms(&self) -> i64 {
        let end = if self.is_terminal() {
            self.updated_at
        } else {
            Utc::now()
        };
        (end - self.created_at).num_milliseconds()
    }
}

impl Default for HealingState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: Option<HealingPhase>,
    pub to: HealingPhase,
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HealingContext {
    pub state: HealingState,
    pub healing_event_name: String,
    pub policy_name: String,
    pub target_pod: String,
    pub target_namespace: String,
    pub correlation_id: Uuid,
}

impl HealingContext {
    pub fn new(
        healing_event_name: String,
        policy_name: String,
        target_pod: String,
        target_namespace: String,
    ) -> Self {
        Self {
            state: HealingState::new(),
            healing_event_name,
            policy_name,
            target_pod,
            target_namespace,
            correlation_id: Uuid::new_v4(),
        }
    }
}
