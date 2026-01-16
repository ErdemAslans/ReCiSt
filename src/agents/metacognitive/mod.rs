mod micro_agent;

use async_trait::async_trait;
use chrono::Utc;
use futures::future::join_all;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{DeleteParams, Patch, PatchParams};
use kube::{Api, Client as KubeClient};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::traits::{Agent, EventHandler};
use crate::clients::llm::LlmClient;
use crate::crd::{ActionType, MetaCognitiveConfig};
use crate::error::{RecistError, Result};
use crate::eventbus::EventBus;
use crate::models::{
    ActionResult, ActionTarget, AgentEvent, AgentEventType, AgentType, DiagnosisHypothesis,
    EventPayload, MicroAgentResult, PlannedAction, ResourceType, RiskLevel, RollbackAction,
    RollbackActionType, RollbackPlan, SolutionStrategy, StrategyType,
};
use micro_agent::MicroAgent;

pub struct MetaCognitiveAgent {
    kube_client: KubeClient,
    llm: Arc<dyn LlmClient>,
    event_bus: EventBus,
    config: MetaCognitiveConfig,
}

impl MetaCognitiveAgent {
    pub async fn new(
        llm: Arc<dyn LlmClient>,
        event_bus: EventBus,
        config: MetaCognitiveConfig,
    ) -> Result<Self> {
        let kube_client = KubeClient::try_default()
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        Ok(Self {
            kube_client,
            llm,
            event_bus,
            config,
        })
    }

    pub async fn determine_strategy(
        &self,
        hypothesis: &DiagnosisHypothesis,
        namespace: &str,
        pod_name: &str,
    ) -> Result<SolutionStrategy> {
        info!(
            "Determining healing strategy for {}/{} with root cause: {}",
            namespace, pod_name, hypothesis.root_cause
        );

        let strategies = self.generate_candidate_strategies(hypothesis);

        let micro_agents: Vec<_> = strategies
            .iter()
            .take(self.config.max_micro_agents as usize)
            .map(|s| {
                MicroAgent::new(
                    s.clone(),
                    hypothesis.clone(),
                    self.llm.clone(),
                    self.config.max_reasoning_depth,
                )
            })
            .collect();

        let futures: Vec<_> = micro_agents
            .into_iter()
            .map(|agent| agent.evaluate())
            .collect();

        let results: Vec<Result<MicroAgentResult>> = join_all(futures).await;

        let mut best_result: Option<MicroAgentResult> = None;
        for result in results {
            match result {
                Ok(r) => {
                    if r.confidence >= self.config.decision_threshold {
                        match &best_result {
                            None => best_result = Some(r),
                            Some(current) if r.confidence > current.confidence => {
                                best_result = Some(r);
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!("Micro-agent evaluation failed: {}", e);
                }
            }
        }

        let selected = best_result.ok_or_else(|| {
            RecistError::HealingError("No strategy met confidence threshold".to_string())
        })?;

        info!(
            "Selected strategy: {:?} with confidence {:.2}",
            selected.strategy_type, selected.confidence
        );

        let mut strategy =
            SolutionStrategy::new(selected.strategy_type.clone(), selected.confidence);

        let action = self.create_action_for_strategy(&selected.strategy_type, namespace, pod_name);
        strategy.add_action(action);

        let rollback = self.create_rollback_plan(&selected.strategy_type, namespace, pod_name);
        strategy.set_rollback_plan(rollback);

        Ok(strategy)
    }

    fn generate_candidate_strategies(&self, hypothesis: &DiagnosisHypothesis) -> Vec<StrategyType> {
        let root_cause_lower = hypothesis.root_cause.to_lowercase();
        let mut strategies = Vec::new();

        if root_cause_lower.contains("memory")
            || root_cause_lower.contains("leak")
            || root_cause_lower.contains("oom")
        {
            strategies.push(StrategyType::PodRestart);
            strategies.push(StrategyType::VerticalScale);
        }

        if root_cause_lower.contains("cpu")
            || root_cause_lower.contains("load")
            || root_cause_lower.contains("capacity")
        {
            strategies.push(StrategyType::HorizontalScale);
            strategies.push(StrategyType::VerticalScale);
        }

        if root_cause_lower.contains("connection")
            || root_cause_lower.contains("pool")
            || root_cause_lower.contains("timeout")
        {
            strategies.push(StrategyType::ConfigUpdate);
            strategies.push(StrategyType::PodRestart);
        }

        if root_cause_lower.contains("dependency")
            || root_cause_lower.contains("upstream")
            || root_cause_lower.contains("downstream")
        {
            strategies.push(StrategyType::DependencyRestart);
            strategies.push(StrategyType::NetworkIsolation);
        }

        if strategies.is_empty() {
            strategies.push(StrategyType::PodRestart);
        }

        strategies
    }

    fn create_action_for_strategy(
        &self,
        strategy: &StrategyType,
        namespace: &str,
        pod_name: &str,
    ) -> PlannedAction {
        let (action_type, resource_type) = match strategy {
            StrategyType::PodRestart => (ActionType::PodRestart, ResourceType::Pod),
            StrategyType::HorizontalScale => {
                (ActionType::HorizontalScale, ResourceType::Deployment)
            }
            StrategyType::VerticalScale => (ActionType::VerticalScale, ResourceType::Deployment),
            StrategyType::ConfigUpdate => (ActionType::ConfigUpdate, ResourceType::ConfigMap),
            StrategyType::DependencyRestart => (ActionType::DependencyRestart, ResourceType::Pod),
            StrategyType::NetworkIsolation => {
                (ActionType::NetworkIsolation, ResourceType::NetworkPolicy)
            }
            StrategyType::Composite => (ActionType::PodRestart, ResourceType::Pod),
        };

        PlannedAction {
            action_type,
            target: ActionTarget {
                resource_type,
                name: pod_name.to_string(),
                namespace: namespace.to_string(),
            },
            parameters: HashMap::new(),
            order: 1,
            depends_on: vec![],
        }
    }

    fn create_rollback_plan(
        &self,
        strategy: &StrategyType,
        namespace: &str,
        pod_name: &str,
    ) -> RollbackPlan {
        let action_type = match strategy {
            StrategyType::HorizontalScale | StrategyType::VerticalScale => {
                RollbackActionType::RestoreReplicas
            }
            StrategyType::ConfigUpdate => RollbackActionType::RestoreConfig,
            StrategyType::NetworkIsolation => RollbackActionType::DeleteNetworkPolicy,
            _ => RollbackActionType::RestartPod,
        };

        RollbackPlan {
            actions: vec![RollbackAction {
                action_type,
                target: ActionTarget {
                    resource_type: ResourceType::Pod,
                    name: pod_name.to_string(),
                    namespace: namespace.to_string(),
                },
                original_state: "{}".to_string(),
            }],
            timeout_seconds: 60,
        }
    }

    pub async fn execute_strategy(
        &self,
        strategy: &SolutionStrategy,
        namespace: &str,
        pod_name: &str,
    ) -> Result<ActionResult> {
        info!(
            "Executing strategy {:?} for {}/{}",
            strategy.strategy_type, namespace, pod_name
        );

        let start = std::time::Instant::now();

        let result = match strategy.strategy_type {
            StrategyType::PodRestart => self.execute_pod_restart(namespace, pod_name).await,
            StrategyType::HorizontalScale => {
                self.execute_horizontal_scale(namespace, pod_name, 2).await
            }
            StrategyType::VerticalScale => self.execute_vertical_scale(namespace, pod_name).await,
            StrategyType::NetworkIsolation => Ok(()),
            _ => self.execute_pod_restart(namespace, pod_name).await,
        };

        let duration = start.elapsed();

        match result {
            Ok(()) => {
                info!(
                    "Strategy {:?} executed successfully in {:?}",
                    strategy.strategy_type, duration
                );
                Ok(ActionResult {
                    action_type: strategy.strategy_type.to_action_type(),
                    success: true,
                    message: format!("Strategy executed successfully in {:?}", duration),
                    executed_at: Utc::now(),
                    duration_ms: duration.as_millis() as i64,
                    rollback_data: None,
                })
            }
            Err(e) => {
                error!("Strategy {:?} failed: {}", strategy.strategy_type, e);
                Ok(ActionResult {
                    action_type: strategy.strategy_type.to_action_type(),
                    success: false,
                    message: format!("Strategy failed: {}", e),
                    executed_at: Utc::now(),
                    duration_ms: duration.as_millis() as i64,
                    rollback_data: None,
                })
            }
        }
    }

    async fn execute_pod_restart(&self, namespace: &str, pod_name: &str) -> Result<()> {
        let pods: Api<Pod> = Api::namespaced(self.kube_client.clone(), namespace);

        pods.delete(pod_name, &DeleteParams::default())
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        info!("Deleted pod {}/{} for restart", namespace, pod_name);
        Ok(())
    }

    async fn execute_horizontal_scale(
        &self,
        namespace: &str,
        pod_name: &str,
        additional_replicas: i32,
    ) -> Result<()> {
        let deployment_name = pod_name
            .rsplit('-')
            .skip(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-");

        let deployments: Api<Deployment> = Api::namespaced(self.kube_client.clone(), namespace);

        let deployment = deployments
            .get(&deployment_name)
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        let current_replicas = deployment
            .spec
            .as_ref()
            .and_then(|s| s.replicas)
            .unwrap_or(1);

        let new_replicas = current_replicas + additional_replicas;

        let patch = serde_json::json!({
            "spec": {
                "replicas": new_replicas
            }
        });

        deployments
            .patch(
                &deployment_name,
                &PatchParams::default(),
                &Patch::Merge(&patch),
            )
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        info!(
            "Scaled deployment {}/{} from {} to {} replicas",
            namespace, deployment_name, current_replicas, new_replicas
        );

        Ok(())
    }

    async fn execute_vertical_scale(&self, namespace: &str, pod_name: &str) -> Result<()> {
        let deployment_name = pod_name
            .rsplit('-')
            .skip(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-");

        let deployments: Api<Deployment> = Api::namespaced(self.kube_client.clone(), namespace);

        let patch = serde_json::json!({
            "spec": {
                "template": {
                    "spec": {
                        "containers": [{
                            "name": deployment_name.clone(),
                            "resources": {
                                "limits": {
                                    "cpu": "1000m",
                                    "memory": "1Gi"
                                },
                                "requests": {
                                    "cpu": "500m",
                                    "memory": "512Mi"
                                }
                            }
                        }]
                    }
                }
            }
        });

        deployments
            .patch(
                &deployment_name,
                &PatchParams::default(),
                &Patch::Strategic(&patch),
            )
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        info!(
            "Updated resource limits for deployment {}/{}",
            namespace, deployment_name
        );

        Ok(())
    }

    pub async fn verify_healing(&self, namespace: &str, pod_name: &str) -> Result<bool> {
        tokio::time::sleep(std::time::Duration::from_secs(
            self.config.verification_wait_seconds,
        ))
        .await;

        let pods: Api<Pod> = Api::namespaced(self.kube_client.clone(), namespace);

        match pods.get(pod_name).await {
            Ok(pod) => {
                let phase = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_ref())
                    .map(|p| p.as_str())
                    .unwrap_or("Unknown");

                let is_running = phase == "Running";

                let all_ready = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.container_statuses.as_ref())
                    .map(|cs| cs.iter().all(|c| c.ready))
                    .unwrap_or(false);

                info!(
                    "Verification for {}/{}: phase={}, all_ready={}",
                    namespace, pod_name, phase, all_ready
                );

                Ok(is_running && all_ready)
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                debug!(
                    "Pod {}/{} not found during verification (may have been recreated)",
                    namespace, pod_name
                );
                Ok(true)
            }
            Err(e) => Err(RecistError::KubeError(e)),
        }
    }
}

#[async_trait]
impl Agent for MetaCognitiveAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::MetaCognitive
    }

    async fn start(&self) -> Result<()> {
        info!("MetaCognitive agent started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("MetaCognitive agent stopped");
        Ok(())
    }

    fn subscribe_to(&self) -> Vec<AgentEventType> {
        vec![AgentEventType::DiagnosisComplete]
    }
}

#[async_trait]
impl EventHandler for MetaCognitiveAgent {
    async fn handle_event(&self, event: AgentEvent) -> Result<Option<AgentEvent>> {
        match &event.payload {
            EventPayload::DiagnosisComplete(payload) => {
                info!(
                    "Received diagnosis, determining strategy for correlation {}",
                    event.correlation_id
                );

                let namespace = "default";
                let pod_name = "unknown";

                match self
                    .determine_strategy(&payload.hypothesis, namespace, pod_name)
                    .await
                {
                    Ok(strategy) => {
                        let result = self
                            .execute_strategy(&strategy, namespace, pod_name)
                            .await?;

                        let success = if result.success {
                            self.verify_healing(namespace, pod_name)
                                .await
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        let response = AgentEvent::healing_complete(
                            event.correlation_id,
                            strategy,
                            success,
                            result.message,
                        );

                        return Ok(Some(response));
                    }
                    Err(e) => {
                        error!("Failed to determine strategy: {}", e);
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }
}
