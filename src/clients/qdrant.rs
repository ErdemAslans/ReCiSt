use qdrant_client::prelude::*;
use qdrant_client::qdrant::{
    value::Kind, vectors_config::Config, Condition, CreateCollection, Distance, FieldCondition,
    Filter, Match, PointStruct, SearchPoints, VectorParams, VectorsConfig,
};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::QdrantConfig;
use crate::error::{RecistError, Result};
use crate::models::{KnowledgeEntry, SimilaritySearchResult};

pub struct QdrantClient {
    client: qdrant_client::Qdrant,
    collection_name: String,
    dimensions: u64,
}

impl QdrantClient {
    pub async fn new(config: &QdrantConfig, dimensions: u32) -> Result<Self> {
        let client = qdrant_client::Qdrant::from_url(&config.url)
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| RecistError::QdrantError(format!("Failed to create client: {}", e)))?;

        let qdrant = Self {
            client,
            collection_name: config.collection_name.clone(),
            dimensions: dimensions as u64,
        };

        qdrant.ensure_collection_exists().await?;

        Ok(qdrant)
    }

    async fn ensure_collection_exists(&self) -> Result<()> {
        let collections =
            self.client.list_collections().await.map_err(|e| {
                RecistError::QdrantError(format!("Failed to list collections: {}", e))
            })?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection_name);

        if !exists {
            info!("Creating Qdrant collection: {}", self.collection_name);

            self.client
                .create_collection(CreateCollection {
                    collection_name: self.collection_name.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: self.dimensions,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .map_err(|e| {
                    RecistError::QdrantError(format!("Failed to create collection: {}", e))
                })?;
        }

        Ok(())
    }

    pub async fn upsert_entry(&self, entry: &KnowledgeEntry) -> Result<()> {
        let embedding = entry
            .embedding
            .as_ref()
            .ok_or_else(|| RecistError::QdrantError("Entry has no embedding".to_string()))?;

        let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
        payload.insert(
            "namespace".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.namespace.clone())),
            },
        );
        payload.insert(
            "pod_name".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.pod_name.clone())),
            },
        );
        payload.insert(
            "error_type".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.error_type.clone())),
            },
        );
        payload.insert(
            "root_cause".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.diagnosis.root_cause.clone())),
            },
        );
        payload.insert(
            "strategy_type".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.solution.strategy_type.clone())),
            },
        );
        payload.insert(
            "success".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::BoolValue(entry.outcome.success)),
            },
        );
        payload.insert(
            "created_at".to_string(),
            qdrant_client::qdrant::Value {
                kind: Some(Kind::StringValue(entry.created_at.to_rfc3339())),
            },
        );

        if let Some(topic) = &entry.topic {
            payload.insert(
                "topic".to_string(),
                qdrant_client::qdrant::Value {
                    kind: Some(Kind::StringValue(topic.clone())),
                },
            );
        }

        let point = PointStruct::new(
            entry.id.to_string(),
            embedding.iter().map(|&x| x as f32).collect::<Vec<_>>(),
            payload,
        );

        self.client
            .upsert_points(&self.collection_name, None, vec![point], None)
            .await
            .map_err(|e| RecistError::QdrantError(format!("Failed to upsert point: {}", e)))?;

        debug!("Upserted knowledge entry: {}", entry.id);
        Ok(())
    }

    pub async fn search_similar(
        &self,
        embedding: &[f32],
        limit: u64,
        namespace_filter: Option<&str>,
        topic_filter: Option<&str>,
    ) -> Result<Vec<ScoredPoint>> {
        let mut filter_conditions = Vec::new();

        if let Some(ns) = namespace_filter {
            filter_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "namespace".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                ns.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        if let Some(topic) = topic_filter {
            filter_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "topic".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                topic.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        let filter = if filter_conditions.is_empty() {
            None
        } else {
            Some(Filter {
                must: filter_conditions,
                ..Default::default()
            })
        };

        let search_result = self
            .client
            .search_points(SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: embedding.to_vec(),
                filter,
                limit,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| RecistError::QdrantError(format!("Search failed: {}", e)))?;

        debug!("Found {} similar entries", search_result.result.len());

        Ok(search_result
            .result
            .into_iter()
            .map(|p| ScoredPoint {
                id: p.id.map(|id| format!("{:?}", id)).unwrap_or_default(),
                score: p.score,
                payload: p.payload,
            })
            .collect())
    }

    pub async fn delete_entry(&self, id: &Uuid) -> Result<()> {
        self.client
            .delete_points(
                &self.collection_name,
                None,
                &qdrant_client::qdrant::PointsSelector {
                    points_selector_one_of: Some(
                        qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                            qdrant_client::qdrant::PointsIdsList {
                                ids: vec![qdrant_client::qdrant::PointId {
                                    point_id_options: Some(
                                        qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                                            id.to_string(),
                                        ),
                                    ),
                                }],
                            },
                        ),
                    ),
                },
                None,
            )
            .await
            .map_err(|e| RecistError::QdrantError(format!("Delete failed: {}", e)))?;

        debug!("Deleted knowledge entry: {}", id);
        Ok(())
    }

    pub async fn get_collection_info(&self) -> Result<CollectionInfo> {
        let info = self
            .client
            .collection_info(&self.collection_name)
            .await
            .map_err(|e| {
                RecistError::QdrantError(format!("Failed to get collection info: {}", e))
            })?;

        Ok(CollectionInfo {
            name: self.collection_name.clone(),
            vectors_count: info
                .result
                .map(|r| r.points_count.unwrap_or(0))
                .unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ScoredPoint {
    pub id: String,
    pub score: f32,
    pub payload: HashMap<String, qdrant_client::qdrant::Value>,
}

#[derive(Debug, Clone)]
pub struct CollectionInfo {
    pub name: String,
    pub vectors_count: u64,
}
