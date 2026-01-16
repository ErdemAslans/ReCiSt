use crate::error::{RecistError, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_namespace")]
    pub namespace: String,

    pub prometheus: PrometheusConfig,
    pub loki: LokiConfig,
    pub qdrant: QdrantConfig,
    pub redis: RedisConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

fn default_namespace() -> String {
    "recist-system".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct PrometheusConfig {
    pub url: String,
    #[serde(default = "default_prometheus_timeout")]
    pub timeout_seconds: u64,
}

fn default_prometheus_timeout() -> u64 {
    10
}

#[derive(Clone, Debug, Deserialize)]
pub struct LokiConfig {
    pub url: String,
    #[serde(default = "default_loki_timeout")]
    pub timeout_seconds: u64,
}

fn default_loki_timeout() -> u64 {
    10
}

#[derive(Clone, Debug, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    #[serde(default = "default_qdrant_timeout")]
    pub timeout_seconds: u64,
}

fn default_qdrant_timeout() -> u64 {
    10
}

#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    #[serde(default = "default_redis_ttl")]
    pub default_ttl_seconds: u64,
}

fn default_redis_ttl() -> u64 {
    3600
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    #[serde(default = "default_metrics_path")]
    pub path: String,
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub json_format: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let prometheus_url = std::env::var("PROMETHEUS_URL")
            .unwrap_or_else(|_| "http://prometheus:9090".to_string());
        let loki_url = std::env::var("LOKI_URL").unwrap_or_else(|_| "http://loki:3100".to_string());
        let qdrant_url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6334".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());

        Ok(Self {
            namespace: std::env::var("NAMESPACE").unwrap_or_else(|_| default_namespace()),
            prometheus: PrometheusConfig {
                url: prometheus_url,
                timeout_seconds: std::env::var("PROMETHEUS_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(default_prometheus_timeout),
            },
            loki: LokiConfig {
                url: loki_url,
                timeout_seconds: std::env::var("LOKI_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(default_loki_timeout),
            },
            qdrant: QdrantConfig {
                url: qdrant_url,
                collection_name: std::env::var("QDRANT_COLLECTION")
                    .unwrap_or_else(|_| "healing_events".to_string()),
                timeout_seconds: std::env::var("QDRANT_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(default_qdrant_timeout),
            },
            redis: RedisConfig {
                url: redis_url,
                default_ttl_seconds: std::env::var("REDIS_TTL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(default_redis_ttl),
            },
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
        })
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| RecistError::ConfigError(format!("Failed to read config file: {}", e)))?;
        serde_yaml::from_str(&contents)
            .map_err(|e| RecistError::ConfigError(format!("Failed to parse config: {}", e)))
    }
}
