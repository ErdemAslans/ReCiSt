use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::config::LokiConfig;
use crate::error::{RecistError, Result};
use crate::models::{LogLevel, StructuredLog};

pub struct LokiClient {
    client: Client,
    base_url: String,
    timeout: Duration,
}

impl LokiClient {
    pub fn new(config: &LokiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| RecistError::LokiError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: config.url.clone(),
            timeout: Duration::from_secs(config.timeout_seconds),
        })
    }

    pub async fn query_logs(
        &self,
        query: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<LogEntry>> {
        debug!("Querying Loki: {} from {} to {}", query, start, end);

        let url = format!("{}/loki/api/v1/query_range", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                (
                    "start",
                    &start.timestamp_nanos_opt().unwrap_or(0).to_string(),
                ),
                ("end", &end.timestamp_nanos_opt().unwrap_or(0).to_string()),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| RecistError::LokiError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RecistError::LokiError(format!(
                "Loki returned error {}: {}",
                status, body
            )));
        }

        let result: LokiQueryResponse = response
            .json()
            .await
            .map_err(|e| RecistError::LokiError(format!("Failed to parse response: {}", e)))?;

        let mut entries = Vec::new();
        if let Some(streams) = result.data.result {
            for stream in streams {
                let labels = stream.stream;
                for (ts, line) in stream.values {
                    let timestamp = parse_loki_timestamp(&ts);
                    entries.push(LogEntry {
                        timestamp,
                        labels: labels.clone(),
                        line,
                    });
                }
            }
        }

        debug!("Loki query returned {} log entries", entries.len());
        Ok(entries)
    }

    pub async fn get_pod_logs(
        &self,
        namespace: &str,
        pod: &str,
        lookback_minutes: u64,
        max_lines: u64,
    ) -> Result<Vec<StructuredLog>> {
        let query = format!(r#"{{namespace="{}", pod="{}"}}"#, namespace, pod);

        let end = Utc::now();
        let start = end - chrono::Duration::minutes(lookback_minutes as i64);

        let entries = self.query_logs(&query, start, end, max_lines).await?;
        let structured = entries
            .into_iter()
            .map(|e| self.parse_log_entry(e, namespace, pod))
            .collect();

        Ok(structured)
    }

    pub async fn get_error_logs(
        &self,
        namespace: &str,
        pod: &str,
        lookback_minutes: u64,
        max_lines: u64,
    ) -> Result<Vec<StructuredLog>> {
        let query = format!(
            r#"{{namespace="{}", pod="{}"}} |~ "(?i)(error|exception|fatal|panic|crash)""#,
            namespace, pod
        );

        let end = Utc::now();
        let start = end - chrono::Duration::minutes(lookback_minutes as i64);

        let entries = self.query_logs(&query, start, end, max_lines).await?;
        let structured = entries
            .into_iter()
            .map(|e| self.parse_log_entry(e, namespace, pod))
            .collect();

        Ok(structured)
    }

    fn parse_log_entry(&self, entry: LogEntry, namespace: &str, pod: &str) -> StructuredLog {
        let level = self.detect_log_level(&entry.line);
        let container_name = entry.labels.get("container").cloned();
        let stack_trace = self.extract_stack_trace(&entry.line);

        StructuredLog {
            timestamp: entry.timestamp,
            level,
            source: entry
                .labels
                .get("app")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            message: entry.line,
            pod_name: pod.to_string(),
            namespace: namespace.to_string(),
            container_name,
            labels: entry.labels,
            stack_trace,
        }
    }

    fn detect_log_level(&self, line: &str) -> LogLevel {
        let line_upper = line.to_uppercase();

        if line_upper.contains("FATAL") || line_upper.contains("PANIC") {
            LogLevel::Fatal
        } else if line_upper.contains("ERROR") || line_upper.contains("EXCEPTION") {
            LogLevel::Error
        } else if line_upper.contains("WARN") {
            LogLevel::Warn
        } else if line_upper.contains("DEBUG") || line_upper.contains("TRACE") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }

    fn extract_stack_trace(&self, line: &str) -> Option<String> {
        let stack_patterns = [
            r"(?s)(at .+\(.+:\d+\).*)+",
            r"(?s)(Traceback \(most recent call last\):.*)",
            r"(?s)(panic:.*goroutine \d+.*)",
        ];

        for pattern in &stack_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(line) {
                    return Some(captures[0].to_string());
                }
            }
        }

        None
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/ready", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                warn!("Loki health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

fn parse_loki_timestamp(ts: &str) -> DateTime<Utc> {
    ts.parse::<i64>()
        .ok()
        .and_then(|nanos| DateTime::from_timestamp_nanos(nanos).into())
        .unwrap_or_else(Utc::now)
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub labels: HashMap<String, String>,
    pub line: String,
}

#[derive(Debug, Deserialize)]
struct LokiQueryResponse {
    status: String,
    data: LokiData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiData {
    result_type: Option<String>,
    result: Option<Vec<LokiStream>>,
}

#[derive(Debug, Deserialize)]
struct LokiStream {
    stream: HashMap<String, String>,
    values: Vec<(String, String)>,
}
