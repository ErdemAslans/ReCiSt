use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::config::RedisConfig;
use crate::error::{RecistError, Result};
use crate::models::KnowledgeEntry;

pub struct RedisClient {
    connection: ConnectionManager,
    default_ttl: Duration,
}

impl RedisClient {
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str()).map_err(|e| RecistError::RedisError(e))?;

        let connection = ConnectionManager::new(client)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        Ok(Self {
            connection,
            default_ttl: Duration::from_secs(config.default_ttl_seconds),
        })
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.default_ttl).await
    }

    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        let mut conn = self.connection.clone();

        conn.set_ex::<_, _, ()>(key, serialized, ttl.as_secs())
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        debug!("Set key {} with TTL {}s", key, ttl.as_secs());
        Ok(())
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.connection.clone();

        let result: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        match result {
            Some(data) => {
                let value: T = serde_json::from_str(&data)?;
                debug!("Got key {}", key);
                Ok(Some(value))
            }
            None => {
                debug!("Key {} not found", key);
                Ok(None)
            }
        }
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.connection.clone();

        let deleted: i32 = conn
            .del(key)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        debug!("Deleted key {}: {}", key, deleted > 0);
        Ok(deleted > 0)
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.connection.clone();

        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        Ok(exists)
    }

    pub async fn lpush<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        let mut conn = self.connection.clone();

        conn.lpush::<_, _, ()>(key, serialized)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        debug!("Pushed to list {}", key);
        Ok(())
    }

    pub async fn lrange<T: DeserializeOwned>(
        &self,
        key: &str,
        start: isize,
        stop: isize,
    ) -> Result<Vec<T>> {
        let mut conn = self.connection.clone();

        let items: Vec<String> = conn
            .lrange(key, start, stop)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        let mut results = Vec::new();
        for item in items {
            match serde_json::from_str(&item) {
                Ok(value) => results.push(value),
                Err(e) => warn!("Failed to deserialize list item: {}", e),
            }
        }

        Ok(results)
    }

    pub async fn ltrim(&self, key: &str, start: isize, stop: isize) -> Result<()> {
        let mut conn = self.connection.clone();

        conn.ltrim::<_, ()>(key, start, stop)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        debug!("Trimmed list {} to [{}, {}]", key, start, stop);
        Ok(())
    }

    pub async fn incr(&self, key: &str) -> Result<i64> {
        let mut conn = self.connection.clone();

        let value: i64 = conn
            .incr(key, 1)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        Ok(value)
    }

    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<bool> {
        let mut conn = self.connection.clone();

        let result: bool = conn
            .expire(key, ttl.as_secs() as i64)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        Ok(result)
    }

    pub async fn ping(&self) -> Result<bool> {
        let mut conn = self.connection.clone();

        let result: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| RecistError::RedisError(e))?;

        Ok(result == "PONG")
    }
}

pub struct LocalKnowledgeCache {
    redis: RedisClient,
    namespace: String,
    max_entries: u64,
}

impl LocalKnowledgeCache {
    pub fn new(redis: RedisClient, namespace: String, max_entries: u64) -> Self {
        Self {
            redis,
            namespace,
            max_entries,
        }
    }

    fn list_key(&self) -> String {
        format!("recist:knowledge:{}:recent", self.namespace)
    }

    fn entry_key(&self, id: &str) -> String {
        format!("recist:knowledge:{}:entry:{}", self.namespace, id)
    }

    pub async fn add_entry(&self, entry: &KnowledgeEntry) -> Result<()> {
        let id = entry.id.to_string();

        self.redis.set(&self.entry_key(&id), entry).await?;
        self.redis.lpush(&self.list_key(), &id).await?;
        self.redis
            .ltrim(&self.list_key(), 0, (self.max_entries - 1) as isize)
            .await?;

        Ok(())
    }

    pub async fn get_recent_entries(&self, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        let ids: Vec<String> = self
            .redis
            .lrange(&self.list_key(), 0, limit as isize - 1)
            .await?;

        let mut entries = Vec::new();
        for id in ids {
            if let Some(entry) = self
                .redis
                .get::<KnowledgeEntry>(&self.entry_key(&id))
                .await?
            {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    pub async fn find_similar_in_cache(&self, error_type: &str) -> Result<Option<KnowledgeEntry>> {
        let entries = self.get_recent_entries(self.max_entries as usize).await?;

        for entry in entries {
            if entry.error_type == error_type && entry.outcome.success {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }
}
