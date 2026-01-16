use chrono::Utc;
use futures::StreamExt;
use kube::{
    api::{Api, ListParams, Patch, PatchParams},
    client::Client,
    runtime::{
        controller::{Action, Controller},
        watcher::Config as WatcherConfig,
    },
    Resource, ResourceExt,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::agents::{
    ContainmentAgent, DiagnosisAgent, EventHandler, KnowledgeAgent, MetaCognitiveAgent,
};
use crate::clients::llm::{create_llm_client, LlmClient};
use crate::clients::{LokiClient, PrometheusClient, QdrantClient, RedisClient};
use crate::config::AppConfig;
use crate::crd::{
    HealingEvent, HealingEventSpec, HealingEventStatus, HealingPhase, PolicyCondition,
    SelfHealingPolicy, SelfHealingPolicyStatus, TriggerReason,
};
use crate::error::{RecistError, Result};
use crate::eventbus::EventBus;
use crate::models::{AgentEvent, AgentEventType, HealingContext};

pub struct ReconcilerContext {
    pub client: Client,
    pub config: AppConfig,
    pub event_bus: EventBus,
    pub containment_agent: Arc<ContainmentAgent>,
    pub diagnosis_agent: Arc<DiagnosisAgent>,
    pub metacognitive_agent: Arc<MetaCognitiveAgent>,
    pub knowledge_agent: Arc<KnowledgeAgent>,
}

impl ReconcilerContext {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let client = Client::try_default()
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        let event_bus = EventBus::new();

        let prometheus = Arc::new(PrometheusClient::new(&config.prometheus)?);
        let loki = Arc::new(LokiClient::new(&config.loki)?);
        let qdrant = Arc::new(QdrantClient::new(&config.qdrant, 1536).await?);
        let redis = Arc::new(RedisClient::new(&config.redis).await?);

        let dummy_llm: Arc<dyn LlmClient> = Arc::new(DummyLlmClient);

        let containment_agent = Arc::new(
            ContainmentAgent::new(
                prometheus.clone(),
                event_bus.clone(),
                Default::default(),
                Default::default(),
            )
            .await?,
        );

        let diagnosis_agent = Arc::new(
            DiagnosisAgent::new(
                prometheus.clone(),
                loki.clone(),
                dummy_llm.clone(),
                event_bus.clone(),
                Default::default(),
            )
            .await?,
        );

        let metacognitive_agent = Arc::new(
            MetaCognitiveAgent::new(dummy_llm.clone(), event_bus.clone(), Default::default())
                .await?,
        );

        let knowledge_agent = Arc::new(
            KnowledgeAgent::new(
                qdrant.clone(),
                redis.clone(),
                dummy_llm.clone(),
                event_bus.clone(),
                Default::default(),
                config.namespace.clone(),
            )
            .await?,
        );

        Ok(Self {
            client,
            config,
            event_bus,
            containment_agent,
            diagnosis_agent,
            metacognitive_agent,
            knowledge_agent,
        })
    }
}

struct DummyLlmClient;

#[async_trait::async_trait]
impl LlmClient for DummyLlmClient {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok("{}".to_string())
    }

    async fn complete_with_system(&self, _system: &str, _prompt: &str) -> Result<String> {
        Ok("{}".to_string())
    }

    async fn diagnose(
        &self,
        _request: &crate::clients::llm::DiagnosisRequest,
    ) -> Result<crate::models::LlmDiagnosisResponse> {
        Ok(crate::models::LlmDiagnosisResponse {
            root_cause: "Unknown".to_string(),
            confidence: 0.5,
            evidence: vec![],
            explanation: "Dummy response".to_string(),
            suggested_actions: vec![],
        })
    }

    async fn evaluate_strategy(
        &self,
        _request: &crate::clients::llm::StrategyEvaluationRequest,
    ) -> Result<crate::models::StrategyEvaluation> {
        Ok(crate::models::StrategyEvaluation {
            strategy_type: crate::models::StrategyType::PodRestart,
            success_probability: 0.7,
            risk_score: 0.3,
            estimated_time_seconds: 30,
            reasoning: "Dummy evaluation".to_string(),
            prerequisites_met: true,
        })
    }

    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.0; 1536])
    }

    fn provider_name(&self) -> &str {
        "Dummy"
    }

    fn model_name(&self) -> &str {
        "dummy-model"
    }
}

pub async fn reconcile_policy(
    policy: Arc<SelfHealingPolicy>,
    ctx: Arc<ReconcilerContext>,
) -> std::result::Result<Action, kube::runtime::controller::Error<RecistError>> {
    let name = policy.name_any();
    let namespace = policy.namespace().unwrap_or_default();

    debug!("Reconciling SelfHealingPolicy {}/{}", namespace, name);

    let api: Api<SelfHealingPolicy> = Api::namespaced(ctx.client.clone(), &namespace);

    let status = SelfHealingPolicyStatus {
        observed_generation: policy.metadata.generation.unwrap_or(0),
        active_healings: 0,
        last_healing_time: None,
        total_healings: 0,
        successful_healings: 0,
        conditions: vec![PolicyCondition {
            condition_type: "Ready".to_string(),
            status: "True".to_string(),
            last_transition_time: Utc::now().to_rfc3339(),
            reason: Some("Reconciled".to_string()),
            message: Some("Policy is ready and monitoring".to_string()),
        }],
    };

    let patch = serde_json::json!({ "status": status });
    let _ = api
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await;

    Ok(Action::requeue(Duration::from_secs(300)))
}

pub fn policy_error(
    policy: Arc<SelfHealingPolicy>,
    error: &kube::runtime::controller::Error<RecistError>,
    _ctx: Arc<ReconcilerContext>,
) -> Action {
    let name = policy.name_any();
    error!("Error reconciling policy {}: {:?}", name, error);
    Action::requeue(Duration::from_secs(60))
}

pub async fn reconcile_event(
    event: Arc<HealingEvent>,
    ctx: Arc<ReconcilerContext>,
) -> std::result::Result<Action, kube::runtime::controller::Error<RecistError>> {
    let name = event.name_any();
    let namespace = event.namespace().unwrap_or_default();

    debug!("Reconciling HealingEvent {}/{}", namespace, name);

    let api: Api<HealingEvent> = Api::namespaced(ctx.client.clone(), &namespace);

    let current_phase = event
        .status
        .as_ref()
        .map(|s| s.phase.clone())
        .unwrap_or(HealingPhase::Pending);

    if matches!(
        current_phase,
        HealingPhase::Completed | HealingPhase::Failed
    ) {
        return Ok(Action::await_change());
    }

    let next_phase = match current_phase {
        HealingPhase::Pending => HealingPhase::Containing,
        HealingPhase::Containing => HealingPhase::Diagnosing,
        HealingPhase::Diagnosing => HealingPhase::Healing,
        HealingPhase::Healing => HealingPhase::Verifying,
        HealingPhase::Verifying => HealingPhase::Completed,
        _ => return Ok(Action::await_change()),
    };

    let mut status = event.status.clone().unwrap_or_default();
    status.phase = next_phase.clone();

    if status.start_time.is_none() {
        status.start_time = Some(Utc::now().to_rfc3339());
    }

    if matches!(next_phase, HealingPhase::Completed | HealingPhase::Failed) {
        status.end_time = Some(Utc::now().to_rfc3339());
    }

    let patch = serde_json::json!({ "status": status });
    let _ = api
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await;

    Ok(Action::requeue(Duration::from_secs(5)))
}

pub fn event_error(
    event: Arc<HealingEvent>,
    error: &kube::runtime::controller::Error<RecistError>,
    _ctx: Arc<ReconcilerContext>,
) -> Action {
    let name = event.name_any();
    error!("Error reconciling healing event {}: {:?}", name, error);
    Action::requeue(Duration::from_secs(30))
}

pub async fn run_controllers(ctx: Arc<ReconcilerContext>) -> Result<()> {
    info!("Starting ReCiSt controllers");

    let policy_api: Api<SelfHealingPolicy> = Api::all(ctx.client.clone());
    let event_api: Api<HealingEvent> = Api::all(ctx.client.clone());

    let policy_controller = Controller::new(policy_api, WatcherConfig::default())
        .run(reconcile_policy, policy_error, ctx.clone())
        .for_each(|res| async move {
            match res {
                Ok(o) => debug!("Reconciled policy: {:?}", o),
                Err(e) => error!("Policy reconciliation error: {:?}", e),
            }
        });

    let event_controller = Controller::new(event_api, WatcherConfig::default())
        .run(reconcile_event, event_error, ctx.clone())
        .for_each(|res| async move {
            match res {
                Ok(o) => debug!("Reconciled event: {:?}", o),
                Err(e) => error!("Event reconciliation error: {:?}", e),
            }
        });

    tokio::select! {
        _ = policy_controller => {
            info!("Policy controller stopped");
        }
        _ = event_controller => {
            info!("Event controller stopped");
        }
    }

    Ok(())
}
