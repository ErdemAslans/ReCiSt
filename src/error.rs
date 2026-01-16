use thiserror::Error;

#[derive(Error, Debug)]
pub enum RecistError {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Prometheus query failed: {0}")]
    PrometheusError(String),

    #[error("Loki query failed: {0}")]
    LokiError(String),

    #[error("LLM request failed: {0}")]
    LlmError(String),

    #[error("Vector database error: {0}")]
    QdrantError(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Agent communication error: {0}")]
    EventBusError(String),

    #[error("Diagnosis failed: {0}")]
    DiagnosisError(String),

    #[error("Healing action failed: {0}")]
    HealingError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, RecistError>;
