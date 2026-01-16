use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::traits::{Agent, EventHandler};
use crate::clients::llm::LlmClient;
use crate::clients::{LocalKnowledgeCache, QdrantClient, RedisClient};
use crate::crd::KnowledgeConfig;
use crate::error::{RecistError, Result};
use crate::eventbus::EventBus;
use crate::models::{
    AgentEvent, AgentEventType, AgentType, DiagnosisSummary, EventPayload, KnowledgeEntry,
    OutcomeSummary, ProactivePrediction, SimilaritySearchResult, SolutionSummary, Topic,
    TrendDirection,
};

pub struct KnowledgeAgent {
    qdrant: Arc<QdrantClient>,
    local_cache: Arc<LocalKnowledgeCache>,
    llm: Arc<dyn LlmClient>,
    event_bus: EventBus,
    config: KnowledgeConfig,
}

impl KnowledgeAgent {
    pub async fn new(
        qdrant: Arc<QdrantClient>,
        redis: Arc<RedisClient>,
        llm: Arc<dyn LlmClient>,
        event_bus: EventBus,
        config: KnowledgeConfig,
        namespace: String,
    ) -> Result<Self> {
        let local_cache = Arc::new(LocalKnowledgeCache::new(
            (*redis).clone(),
            namespace,
            config.max_local_events,
        ));

        Ok(Self {
            qdrant,
            local_cache,
            llm,
            event_bus,
            config,
        })
    }

    pub async fn record_healing_event(
        &self,
        namespace: &str,
        pod_name: &str,
        error_type: &str,
        diagnosis: DiagnosisSummary,
        solution: SolutionSummary,
        outcome: OutcomeSummary,
    ) -> Result<KnowledgeEntry> {
        info!(
            "Recording healing event for {}/{}: {}",
            namespace, pod_name, error_type
        );

        let mut entry = KnowledgeEntry::new(
            namespace.to_string(),
            pod_name.to_string(),
            error_type.to_string(),
            diagnosis,
            solution,
            outcome,
        );

        entry.set_ttl_days(self.config.knowledge_ttl_days);

        let summary = entry.summary_text();
        match self.llm.generate_embedding(&summary).await {
            Ok(embedding) => {
                entry.set_embedding(embedding);
            }
            Err(e) => {
                warn!("Failed to generate embedding: {}", e);
            }
        }

        if entry.embedding.is_some() {
            let topic = self.determine_topic(&entry).await?;
            entry.set_topic(topic);
        }

        if entry.embedding.is_some() {
            self.qdrant.upsert_entry(&entry).await?;
        }

        self.local_cache.add_entry(&entry).await?;

        info!("Recorded knowledge entry: {}", entry.id);

        let event = AgentEvent::knowledge_updated(Uuid::new_v4(), entry.clone());
        self.event_bus.publish(event).await?;

        Ok(entry)
    }

    pub async fn find_similar_events(
        &self,
        error_type: &str,
        namespace: Option<&str>,
        limit: u64,
    ) -> Result<Vec<SimilaritySearchResult>> {
        if let Some(cached) = self.local_cache.find_similar_in_cache(error_type).await? {
            info!("Found similar event in local cache: {}", cached.id);
            return Ok(vec![SimilaritySearchResult {
                entry: cached,
                similarity_score: 1.0,
            }]);
        }

        let embedding = self.llm.generate_embedding(error_type).await?;

        let results = self
            .qdrant
            .search_similar(&embedding, limit, namespace, None)
            .await?;

        let mut similar_entries = Vec::new();
        for point in results {
            if point.score >= self.config.similarity_threshold as f32 {
                let entry = self.point_to_entry(&point)?;
                similar_entries.push(SimilaritySearchResult {
                    entry,
                    similarity_score: point.score,
                });
            }
        }

        info!(
            "Found {} similar events for error type: {}",
            similar_entries.len(),
            error_type
        );

        Ok(similar_entries)
    }

    pub async fn get_recommended_strategy(
        &self,
        error_type: &str,
        namespace: &str,
    ) -> Result<Option<String>> {
        let similar = self
            .find_similar_events(error_type, Some(namespace), 5)
            .await?;

        let successful: Vec<_> = similar
            .into_iter()
            .filter(|s| s.entry.outcome.success)
            .collect();

        if successful.is_empty() {
            return Ok(None);
        }

        let best = successful.into_iter().max_by(|a, b| {
            a.entry
                .success_rate
                .partial_cmp(&b.entry.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(best.map(|s| s.entry.solution.strategy_type))
    }

    async fn determine_topic(&self, entry: &KnowledgeEntry) -> Result<String> {
        let root_cause_lower = entry.diagnosis.root_cause.to_lowercase();

        let topic = if root_cause_lower.contains("memory")
            || root_cause_lower.contains("oom")
            || root_cause_lower.contains("leak")
        {
            "memory_issues"
        } else if root_cause_lower.contains("cpu")
            || root_cause_lower.contains("load")
            || root_cause_lower.contains("capacity")
        {
            "resource_saturation"
        } else if root_cause_lower.contains("connection")
            || root_cause_lower.contains("network")
            || root_cause_lower.contains("timeout")
        {
            "network_issues"
        } else if root_cause_lower.contains("database")
            || root_cause_lower.contains("query")
            || root_cause_lower.contains("sql")
        {
            "database_issues"
        } else if root_cause_lower.contains("dependency")
            || root_cause_lower.contains("upstream")
            || root_cause_lower.contains("downstream")
        {
            "dependency_issues"
        } else if root_cause_lower.contains("config") || root_cause_lower.contains("configuration")
        {
            "configuration_issues"
        } else {
            "general"
        };

        Ok(topic.to_string())
    }

    fn point_to_entry(
        &self,
        point: &crate::clients::qdrant::ScoredPoint,
    ) -> Result<KnowledgeEntry> {
        let get_string = |key: &str| -> String {
            point
                .payload
                .get(key)
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default()
        };

        let get_bool = |key: &str| -> bool {
            point
                .payload
                .get(key)
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::BoolValue(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false)
        };

        Ok(KnowledgeEntry {
            id: Uuid::parse_str(&point.id).unwrap_or_else(|_| Uuid::new_v4()),
            namespace: get_string("namespace"),
            pod_name: get_string("pod_name"),
            error_type: get_string("error_type"),
            diagnosis: DiagnosisSummary {
                hypothesis: String::new(),
                confidence: 0.0,
                root_cause: get_string("root_cause"),
                key_evidence: vec![],
            },
            solution: SolutionSummary {
                strategy_type: get_string("strategy_type"),
                actions: vec![],
                duration_ms: 0,
            },
            outcome: OutcomeSummary {
                success: get_bool("success"),
                message: String::new(),
                total_duration_ms: 0,
            },
            embedding: None,
            topic: point
                .payload
                .get("topic")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
                    _ => None,
                }),
            created_at: Utc::now(),
            expires_at: None,
            usage_count: 0,
            success_rate: 0.0,
        })
    }

    pub async fn cleanup_expired_entries(&self) -> Result<u64> {
        info!("Cleaning up expired knowledge entries");
        Ok(0)
    }
}

#[async_trait]
impl Agent for KnowledgeAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Knowledge
    }

    async fn start(&self) -> Result<()> {
        info!("Knowledge agent started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("Knowledge agent stopped");
        Ok(())
    }

    fn subscribe_to(&self) -> Vec<AgentEventType> {
        vec![AgentEventType::HealingComplete]
    }
}

#[async_trait]
impl EventHandler for KnowledgeAgent {
    async fn handle_event(&self, event: AgentEvent) -> Result<Option<AgentEvent>> {
        match &event.payload {
            EventPayload::HealingComplete(payload) => {
                info!(
                    "Received healing complete event for correlation {}",
                    event.correlation_id
                );

                let diagnosis_summary = DiagnosisSummary {
                    hypothesis: String::new(),
                    confidence: payload.strategy.confidence,
                    root_cause: String::new(),
                    key_evidence: vec![],
                };

                let solution_summary = SolutionSummary::from(&payload.strategy);

                let outcome_summary = OutcomeSummary {
                    success: payload.success,
                    message: payload.message.clone(),
                    total_duration_ms: 0,
                };

                if let Err(e) = self
                    .record_healing_event(
                        "default",
                        "unknown",
                        &payload.strategy.strategy_type.to_string(),
                        diagnosis_summary,
                        solution_summary,
                        outcome_summary,
                    )
                    .await
                {
                    error!("Failed to record healing event: {}", e);
                }
            }
            _ => {}
        }

        Ok(None)
    }
}
