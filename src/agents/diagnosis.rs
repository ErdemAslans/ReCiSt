use async_trait::async_trait;
use chrono::Utc;
use k8s_openapi::api::core::v1::Event as K8sEvent;
use kube::Api;
use kube::Client as KubeClient;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::traits::{Agent, EventHandler};
use crate::clients::llm::{DiagnosisRequest, LlmClient, MetricSnapshot};
use crate::clients::{LokiClient, PrometheusClient};
use crate::crd::DiagnosisConfig;
use crate::error::{RecistError, Result};
use crate::eventbus::EventBus;
use crate::models::{
    AgentEvent, AgentEventType, AgentType, CausalNode, CausalNodeType, CausalRelation, CausalTree,
    DiagnosisHypothesis, EventPayload, Evidence, EvidenceSource, FaultCluster, StructuredLog,
};

pub struct DiagnosisAgent {
    kube_client: KubeClient,
    prometheus: Arc<PrometheusClient>,
    loki: Arc<LokiClient>,
    llm: Arc<dyn LlmClient>,
    event_bus: EventBus,
    config: DiagnosisConfig,
}

impl DiagnosisAgent {
    pub async fn new(
        prometheus: Arc<PrometheusClient>,
        loki: Arc<LokiClient>,
        llm: Arc<dyn LlmClient>,
        event_bus: EventBus,
        config: DiagnosisConfig,
    ) -> Result<Self> {
        let kube_client = KubeClient::try_default()
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        Ok(Self {
            kube_client,
            prometheus,
            loki,
            llm,
            event_bus,
            config,
        })
    }

    pub async fn diagnose(&self, fault_cluster: &FaultCluster) -> Result<DiagnosisHypothesis> {
        let fault = fault_cluster
            .primary_fault()
            .ok_or_else(|| RecistError::DiagnosisError("No faults in cluster".to_string()))?;

        info!(
            "Starting diagnosis for pod {}/{}",
            fault.namespace, fault.pod_name
        );

        let logs = self.collect_logs(&fault.namespace, &fault.pod_name).await?;
        let metrics = self
            .collect_metrics(&fault.namespace, &fault.pod_name)
            .await?;
        let k8s_events = self
            .collect_kubernetes_events(&fault.namespace, &fault.pod_name)
            .await?;

        let causal_tree = self.build_causal_tree(&logs, &metrics, &k8s_events);

        let llm_request = DiagnosisRequest {
            logs: logs.iter().map(|l| l.message.clone()).collect(),
            metrics: metrics
                .iter()
                .map(|(name, value)| MetricSnapshot {
                    name: name.clone(),
                    value: *value,
                    threshold: None,
                })
                .collect(),
            kubernetes_events: k8s_events,
            pod_name: fault.pod_name.clone(),
            namespace: fault.namespace.clone(),
            error_type: fault.primary_reason().to_string(),
        };

        let llm_response = self.llm.diagnose(&llm_request).await?;

        let mut hypothesis = DiagnosisHypothesis::new(
            llm_response.explanation.clone(),
            llm_response.confidence,
            llm_response.root_cause.clone(),
        );

        for evidence_text in &llm_response.evidence {
            hypothesis.add_evidence(Evidence {
                source: EvidenceSource::Log,
                content: evidence_text.clone(),
                timestamp: Utc::now(),
                relevance_score: 0.8,
            });
        }

        hypothesis.causal_tree = causal_tree;

        info!(
            "Diagnosis complete for {}/{}: {} (confidence: {:.2})",
            fault.namespace, fault.pod_name, hypothesis.root_cause, hypothesis.confidence
        );

        if !hypothesis.meets_threshold(self.config.confidence_threshold) {
            warn!(
                "Diagnosis confidence {:.2} below threshold {:.2}",
                hypothesis.confidence, self.config.confidence_threshold
            );
        }

        Ok(hypothesis)
    }

    async fn collect_logs(&self, namespace: &str, pod_name: &str) -> Result<Vec<StructuredLog>> {
        let error_logs = self
            .loki
            .get_error_logs(
                namespace,
                pod_name,
                self.config.log_lookback_minutes,
                self.config.max_log_lines / 2,
            )
            .await?;

        let all_logs = self
            .loki
            .get_pod_logs(
                namespace,
                pod_name,
                self.config.log_lookback_minutes,
                self.config.max_log_lines / 2,
            )
            .await?;

        let mut combined: Vec<StructuredLog> = error_logs;
        for log in all_logs {
            if !combined
                .iter()
                .any(|l| l.message == log.message && l.timestamp == log.timestamp)
            {
                combined.push(log);
            }
        }

        combined.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        debug!(
            "Collected {} logs for {}/{}",
            combined.len(),
            namespace,
            pod_name
        );

        Ok(combined)
    }

    async fn collect_metrics(
        &self,
        namespace: &str,
        pod_name: &str,
    ) -> Result<HashMap<String, f64>> {
        let mut metrics = HashMap::new();

        let cpu = self
            .prometheus
            .get_pod_cpu_usage(namespace, pod_name)
            .await?;
        metrics.insert("cpu_usage".to_string(), cpu);

        let memory = self
            .prometheus
            .get_pod_memory_usage(namespace, pod_name)
            .await?;
        metrics.insert("memory_usage".to_string(), memory);

        let error_rate = self
            .prometheus
            .get_pod_error_rate(namespace, pod_name)
            .await?;
        metrics.insert("error_rate".to_string(), error_rate);

        let latency = self
            .prometheus
            .get_pod_latency_p99(namespace, pod_name)
            .await?;
        metrics.insert("latency_p99_ms".to_string(), latency);

        debug!(
            "Collected {} metrics for {}/{}",
            metrics.len(),
            namespace,
            pod_name
        );

        Ok(metrics)
    }

    async fn collect_kubernetes_events(
        &self,
        namespace: &str,
        pod_name: &str,
    ) -> Result<Vec<String>> {
        let events_api: Api<K8sEvent> = Api::namespaced(self.kube_client.clone(), namespace);

        let events = events_api
            .list(&Default::default())
            .await
            .map_err(|e| RecistError::KubeError(e))?;

        let relevant_events: Vec<String> = events
            .items
            .into_iter()
            .filter(|e| {
                e.involved_object
                    .name
                    .as_ref()
                    .map(|n| n == pod_name)
                    .unwrap_or(false)
            })
            .map(|e| {
                format!(
                    "[{}] {}: {}",
                    e.type_.unwrap_or_default(),
                    e.reason.unwrap_or_default(),
                    e.message.unwrap_or_default()
                )
            })
            .collect();

        debug!(
            "Collected {} Kubernetes events for {}/{}",
            relevant_events.len(),
            namespace,
            pod_name
        );

        Ok(relevant_events)
    }

    fn build_causal_tree(
        &self,
        logs: &[StructuredLog],
        metrics: &HashMap<String, f64>,
        k8s_events: &[String],
    ) -> CausalTree {
        let mut tree = CausalTree::new();
        let mut node_ids: Vec<String> = Vec::new();

        for (i, log) in logs.iter().enumerate().take(20) {
            let node_type = match log.level {
                crate::models::LogLevel::Error | crate::models::LogLevel::Fatal => {
                    CausalNodeType::Error
                }
                crate::models::LogLevel::Warn => CausalNodeType::Warning,
                _ => CausalNodeType::Symptom,
            };

            let id = format!("log_{}", i);
            let node = CausalNode::new(
                id.clone(),
                node_type,
                log.message.chars().take(200).collect(),
                "log".to_string(),
            );

            tree.add_node(node);
            node_ids.push(id);
        }

        for (name, value) in metrics {
            let id = format!("metric_{}", name);
            let node = CausalNode::new(
                id.clone(),
                CausalNodeType::Metric,
                format!("{}: {:.2}", name, value),
                "prometheus".to_string(),
            );

            tree.add_node(node);
            node_ids.push(id);
        }

        for (i, event) in k8s_events.iter().enumerate() {
            let id = format!("event_{}", i);
            let node = CausalNode::new(
                id.clone(),
                CausalNodeType::Event,
                event.clone(),
                "kubernetes".to_string(),
            );

            tree.add_node(node);
            node_ids.push(id);
        }

        for i in 1..node_ids.len().min(10) {
            tree.add_edge(
                node_ids[i - 1].clone(),
                node_ids[i].clone(),
                CausalRelation::Precedes,
            );
        }

        if !node_ids.is_empty() {
            tree.set_root(node_ids[0].clone());
        }

        tree
    }
}

#[async_trait]
impl Agent for DiagnosisAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Diagnosis
    }

    async fn start(&self) -> Result<()> {
        info!("Diagnosis agent started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("Diagnosis agent stopped");
        Ok(())
    }

    fn subscribe_to(&self) -> Vec<AgentEventType> {
        vec![
            AgentEventType::FaultDetected,
            AgentEventType::ContainmentComplete,
        ]
    }
}

#[async_trait]
impl EventHandler for DiagnosisAgent {
    async fn handle_event(&self, event: AgentEvent) -> Result<Option<AgentEvent>> {
        match &event.payload {
            EventPayload::FaultDetected(payload) => {
                info!(
                    "Received fault detection event, starting diagnosis for correlation {}",
                    event.correlation_id
                );

                match self.diagnose(&payload.fault_cluster).await {
                    Ok(hypothesis) => {
                        let response =
                            AgentEvent::diagnosis_complete(event.correlation_id, hypothesis);
                        return Ok(Some(response));
                    }
                    Err(e) => {
                        error!("Diagnosis failed: {}", e);
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }
}
