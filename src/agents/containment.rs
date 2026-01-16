use async_trait::async_trait;
use chrono::Utc;
use k8s_openapi::api::networking::v1::{
    NetworkPolicy, NetworkPolicyEgressRule, NetworkPolicyIngressRule, NetworkPolicySpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client as KubeClient};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::traits::{Agent, EventHandler};
use crate::clients::PrometheusClient;
use crate::crd::{
    ContainmentConfig, IsolationStrategy, SelfHealingPolicySpec, Thresholds, TriggerReason,
};
use crate::error::{RecistError, Result};
use crate::eventbus::EventBus;
use crate::models::{
    AcceptingNeighbor, AgentEvent, AgentEventType, AgentType, Fault, FaultCluster, FaultSeverity,
    IsolationRule, IsolationRuleType, NeighborNegotiationResult, RejectedNeighbor, TrafficRedirect,
};

pub struct ContainmentAgent {
    kube_client: KubeClient,
    prometheus: Arc<PrometheusClient>,
    event_bus: EventBus,
    config: ContainmentConfig,
    thresholds: Thresholds,
    active_isolations: Arc<RwLock<HashMap<String, IsolationRule>>>,
    running: Arc<RwLock<bool>>,
}

impl ContainmentAgent {
    pub async fn new(
        prometheus: Arc<PrometheusClient>,
        event_bus: EventBus,
        config: ContainmentConfig,
        thresholds: Thresholds,
    ) -> Result<Self> {
        let kube_client = KubeClient::try_default()
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        Ok(Self {
            kube_client,
            prometheus,
            event_bus,
            config,
            thresholds,
            active_isolations: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn check_metrics(&self, namespace: &str) -> Result<FaultCluster> {
        let metrics = self.prometheus.get_all_pod_metrics(namespace).await?;
        let mut fault_cluster = FaultCluster::new(namespace.to_string());

        for pod_metrics in metrics {
            let mut reasons = Vec::new();

            if pod_metrics.cpu_usage > self.thresholds.cpu {
                reasons.push(TriggerReason::HighCpu);
            }
            if pod_metrics.memory_usage > self.thresholds.memory {
                reasons.push(TriggerReason::HighMemory);
            }
            if pod_metrics.error_rate > self.thresholds.error_rate {
                reasons.push(TriggerReason::HighErrorRate);
            }
            if pod_metrics.latency_ms > self.thresholds.latency_ms as f64 {
                reasons.push(TriggerReason::HighLatency);
            }

            if !reasons.is_empty() {
                let fault = Fault::new(
                    pod_metrics.pod_name.clone(),
                    namespace.to_string(),
                    reasons,
                    crate::crd::TriggerMetrics {
                        cpu_usage: Some(pod_metrics.cpu_usage),
                        memory_usage: Some(pod_metrics.memory_usage),
                        error_rate: Some(pod_metrics.error_rate),
                        latency_ms: Some(pod_metrics.latency_ms as u64),
                        restart_count: None,
                    },
                );

                info!(
                    "Fault detected in pod {}/{}: {:?}",
                    namespace, pod_metrics.pod_name, fault.reasons
                );

                fault_cluster.add_fault(fault);
            }
        }

        Ok(fault_cluster)
    }

    pub async fn isolate_pod(&self, fault: &Fault) -> Result<IsolationRule> {
        let strategy = self.determine_isolation_strategy(fault);
        let policy_name = format!("recist-isolate-{}", fault.pod_name);

        info!(
            "Isolating pod {}/{} with strategy {:?}",
            fault.namespace, fault.pod_name, strategy
        );

        let network_policy = self.create_network_policy(&policy_name, &fault.pod_name, &strategy);
        let api: Api<NetworkPolicy> = Api::namespaced(self.kube_client.clone(), &fault.namespace);

        match api.create(&PostParams::default(), &network_policy).await {
            Ok(_) => {
                info!(
                    "Created NetworkPolicy {} for pod {}",
                    policy_name, fault.pod_name
                );
            }
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                debug!("NetworkPolicy {} already exists, updating", policy_name);
                let _ = api.delete(&policy_name, &DeleteParams::default()).await;
                api.create(&PostParams::default(), &network_policy).await?;
            }
            Err(e) => return Err(RecistError::KubeError(e)),
        }

        let rule = IsolationRule {
            pod_name: fault.pod_name.clone(),
            namespace: fault.namespace.clone(),
            network_policy_name: policy_name,
            created_at: Utc::now(),
            rule_type: match strategy {
                IsolationStrategy::Hard => IsolationRuleType::DenyAll,
                IsolationStrategy::Soft => IsolationRuleType::DenyIngress,
                IsolationStrategy::Auto => IsolationRuleType::DenyIngress,
            },
        };

        let mut isolations = self.active_isolations.write().await;
        isolations.insert(fault.pod_name.clone(), rule.clone());

        Ok(rule)
    }

    pub async fn remove_isolation(&self, pod_name: &str, namespace: &str) -> Result<()> {
        let policy_name = format!("recist-isolate-{}", pod_name);
        let api: Api<NetworkPolicy> = Api::namespaced(self.kube_client.clone(), namespace);

        match api.delete(&policy_name, &DeleteParams::default()).await {
            Ok(_) => {
                info!("Removed NetworkPolicy {} for pod {}", policy_name, pod_name);
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                debug!("NetworkPolicy {} not found, already removed", policy_name);
            }
            Err(e) => return Err(RecistError::KubeError(e)),
        }

        let mut isolations = self.active_isolations.write().await;
        isolations.remove(pod_name);

        Ok(())
    }

    pub async fn negotiate_with_neighbors(
        &self,
        faulty_pod: &str,
        namespace: &str,
    ) -> Result<NeighborNegotiationResult> {
        let all_metrics = self.prometheus.get_all_pod_metrics(namespace).await?;

        let mut accepting = Vec::new();
        let mut rejected = Vec::new();

        for pod_metrics in all_metrics {
            if pod_metrics.pod_name == faulty_pod {
                continue;
            }

            let available_capacity = 1.0 - pod_metrics.cpu_usage.max(pod_metrics.memory_usage);

            if available_capacity >= self.config.neighbor_capacity_threshold {
                let load_fraction = (available_capacity - self.config.neighbor_capacity_threshold)
                    / (1.0 - self.config.neighbor_capacity_threshold);

                accepting.push(AcceptingNeighbor {
                    pod_name: pod_metrics.pod_name,
                    available_capacity,
                    accepted_load_fraction: load_fraction.min(0.5),
                });
            } else {
                rejected.push(RejectedNeighbor {
                    pod_name: pod_metrics.pod_name,
                    reason: format!(
                        "Insufficient capacity: {:.2}% available",
                        available_capacity * 100.0
                    ),
                });
            }
        }

        info!(
            "Neighbor negotiation for {}: {} accepting, {} rejected",
            faulty_pod,
            accepting.len(),
            rejected.len()
        );

        Ok(NeighborNegotiationResult {
            requesting_pod: faulty_pod.to_string(),
            accepting_pods: accepting,
            rejected_pods: rejected,
        })
    }

    fn determine_isolation_strategy(&self, fault: &Fault) -> IsolationStrategy {
        match &self.config.isolation_strategy {
            IsolationStrategy::Auto => {
                if fault.severity >= FaultSeverity::Critical {
                    IsolationStrategy::Hard
                } else {
                    IsolationStrategy::Soft
                }
            }
            strategy => strategy.clone(),
        }
    }

    fn create_network_policy(
        &self,
        name: &str,
        pod_name: &str,
        strategy: &IsolationStrategy,
    ) -> NetworkPolicy {
        let mut labels = std::collections::BTreeMap::new();
        labels.insert(
            "statefulset.kubernetes.io/pod-name".to_string(),
            pod_name.to_string(),
        );

        let mut policy_types = vec!["Ingress".to_string()];
        if matches!(strategy, IsolationStrategy::Hard) {
            policy_types.push("Egress".to_string());
        }

        NetworkPolicy {
            metadata: kube::api::ObjectMeta {
                name: Some(name.to_string()),
                labels: Some({
                    let mut l = std::collections::BTreeMap::new();
                    l.insert(
                        "app.kubernetes.io/managed-by".to_string(),
                        "recist".to_string(),
                    );
                    l
                }),
                ..Default::default()
            },
            spec: Some(NetworkPolicySpec {
                pod_selector: LabelSelector {
                    match_labels: Some(labels),
                    ..Default::default()
                },
                policy_types: Some(policy_types),
                ingress: Some(vec![]),
                egress: if matches!(strategy, IsolationStrategy::Hard) {
                    Some(vec![])
                } else {
                    None
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub async fn run_check_loop(&self, namespaces: Vec<String>) {
        let interval = std::time::Duration::from_secs(self.config.check_interval_seconds);

        loop {
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            for namespace in &namespaces {
                match self.check_metrics(namespace).await {
                    Ok(fault_cluster) => {
                        if !fault_cluster.is_empty() {
                            for fault in &fault_cluster.faults {
                                if let Err(e) = self.isolate_pod(fault).await {
                                    error!("Failed to isolate pod {}: {}", fault.pod_name, e);
                                }
                            }

                            let correlation_id = Uuid::new_v4();
                            let event =
                                AgentEvent::fault_detected(correlation_id, fault_cluster.clone());

                            if let Err(e) = self.event_bus.publish(event).await {
                                error!("Failed to publish fault event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check metrics for namespace {}: {}", namespace, e);
                    }
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}

#[async_trait]
impl Agent for ContainmentAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Containment
    }

    async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        info!("Containment agent started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Containment agent stopped");
        Ok(())
    }

    fn subscribe_to(&self) -> Vec<AgentEventType> {
        vec![AgentEventType::HealingComplete]
    }
}

#[async_trait]
impl EventHandler for ContainmentAgent {
    async fn handle_event(&self, event: AgentEvent) -> Result<Option<AgentEvent>> {
        match &event.payload {
            crate::models::EventPayload::HealingComplete(payload) => {
                if payload.success {
                    info!(
                        "Healing complete, removing isolation for correlation {}",
                        event.correlation_id
                    );
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
