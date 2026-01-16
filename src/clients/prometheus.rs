use chrono::{DateTime, Utc};
use prometheus_http_query::{Client, InstantVector, RangeVector};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info};

use crate::config::PrometheusConfig;
use crate::error::{RecistError, Result};

pub struct PrometheusClient {
    client: Client,
    timeout: Duration,
}

impl PrometheusClient {
    pub fn new(config: &PrometheusConfig) -> Result<Self> {
        let client = Client::try_from(config.url.as_str())
            .map_err(|e| RecistError::PrometheusError(format!("Failed to create client: {}", e)))?;

        Ok(Self {
            client,
            timeout: Duration::from_secs(config.timeout_seconds),
        })
    }

    pub async fn query_instant(&self, query: &str) -> Result<Vec<MetricSample>> {
        debug!("Executing instant query: {}", query);

        let response = self
            .client
            .query(query)
            .timeout(self.timeout)
            .get()
            .await
            .map_err(|e| RecistError::PrometheusError(format!("Query failed: {}", e)))?;

        let samples = match response.data() {
            prometheus_http_query::response::Data::Vector(v) => v
                .iter()
                .map(|sample| {
                    let labels: HashMap<String, String> = sample
                        .metric()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();

                    MetricSample {
                        labels,
                        value: sample.sample().value(),
                        timestamp: DateTime::from_timestamp(sample.sample().timestamp() as i64, 0)
                            .unwrap_or_else(Utc::now),
                    }
                })
                .collect(),
            _ => Vec::new(),
        };

        debug!("Query returned {} samples", samples.len());
        Ok(samples)
    }

    pub async fn query_range(
        &self,
        query: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step_seconds: u64,
    ) -> Result<Vec<MetricTimeSeries>> {
        debug!("Executing range query: {} from {} to {}", query, start, end);

        let response = self
            .client
            .query_range(
                query,
                start.timestamp(),
                end.timestamp(),
                step_seconds as f64,
            )
            .timeout(self.timeout)
            .get()
            .await
            .map_err(|e| RecistError::PrometheusError(format!("Range query failed: {}", e)))?;

        let series = match response.data() {
            prometheus_http_query::response::Data::Matrix(m) => m
                .iter()
                .map(|ts| {
                    let labels: HashMap<String, String> = ts
                        .metric()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();

                    let values: Vec<(DateTime<Utc>, f64)> = ts
                        .samples()
                        .iter()
                        .map(|s| {
                            (
                                DateTime::from_timestamp(s.timestamp() as i64, 0)
                                    .unwrap_or_else(Utc::now),
                                s.value(),
                            )
                        })
                        .collect();

                    MetricTimeSeries { labels, values }
                })
                .collect(),
            _ => Vec::new(),
        };

        debug!("Range query returned {} series", series.len());
        Ok(series)
    }

    pub async fn get_pod_cpu_usage(&self, namespace: &str, pod: &str) -> Result<f64> {
        let query = format!(
            r#"sum(rate(container_cpu_usage_seconds_total{{namespace="{}", pod="{}"}}[5m])) by (pod)"#,
            namespace, pod
        );

        let samples = self.query_instant(&query).await?;
        Ok(samples.first().map(|s| s.value).unwrap_or(0.0))
    }

    pub async fn get_pod_memory_usage(&self, namespace: &str, pod: &str) -> Result<f64> {
        let query = format!(
            r#"sum(container_memory_usage_bytes{{namespace="{}", pod="{}"}}) by (pod) / sum(container_spec_memory_limit_bytes{{namespace="{}", pod="{}"}}) by (pod)"#,
            namespace, pod, namespace, pod
        );

        let samples = self.query_instant(&query).await?;
        Ok(samples.first().map(|s| s.value).unwrap_or(0.0))
    }

    pub async fn get_pod_error_rate(&self, namespace: &str, pod: &str) -> Result<f64> {
        let query = format!(
            r#"sum(rate(http_requests_total{{namespace="{}", pod="{}", status=~"5.."}}[5m])) / sum(rate(http_requests_total{{namespace="{}", pod="{}"}}[5m]))"#,
            namespace, pod, namespace, pod
        );

        let samples = self.query_instant(&query).await?;
        let value = samples.first().map(|s| s.value).unwrap_or(0.0);
        Ok(if value.is_nan() { 0.0 } else { value })
    }

    pub async fn get_pod_latency_p99(&self, namespace: &str, pod: &str) -> Result<f64> {
        let query = format!(
            r#"histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket{{namespace="{}", pod="{}"}}[5m])) by (le))"#,
            namespace, pod
        );

        let samples = self.query_instant(&query).await?;
        let latency_seconds = samples.first().map(|s| s.value).unwrap_or(0.0);
        Ok(latency_seconds * 1000.0)
    }

    pub async fn get_all_pod_metrics(&self, namespace: &str) -> Result<Vec<PodMetrics>> {
        let cpu_query = format!(
            r#"sum(rate(container_cpu_usage_seconds_total{{namespace="{}"}}[5m])) by (pod)"#,
            namespace
        );

        let memory_query = format!(
            r#"sum(container_memory_usage_bytes{{namespace="{}"}}) by (pod) / sum(container_spec_memory_limit_bytes{{namespace="{}"}}) by (pod)"#,
            namespace, namespace
        );

        let cpu_samples = self.query_instant(&cpu_query).await?;
        let memory_samples = self.query_instant(&memory_query).await?;

        let mut metrics_map: HashMap<String, PodMetrics> = HashMap::new();

        for sample in cpu_samples {
            if let Some(pod_name) = sample.labels.get("pod") {
                metrics_map
                    .entry(pod_name.clone())
                    .or_insert_with(|| PodMetrics {
                        pod_name: pod_name.clone(),
                        namespace: namespace.to_string(),
                        cpu_usage: 0.0,
                        memory_usage: 0.0,
                        error_rate: 0.0,
                        latency_ms: 0.0,
                    })
                    .cpu_usage = sample.value;
            }
        }

        for sample in memory_samples {
            if let Some(pod_name) = sample.labels.get("pod") {
                if let Some(metrics) = metrics_map.get_mut(pod_name) {
                    let value = sample.value;
                    metrics.memory_usage = if value.is_nan() { 0.0 } else { value };
                }
            }
        }

        Ok(metrics_map.into_values().collect())
    }
}

#[derive(Clone, Debug)]
pub struct MetricSample {
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MetricTimeSeries {
    pub labels: HashMap<String, String>,
    pub values: Vec<(DateTime<Utc>, f64)>,
}

#[derive(Clone, Debug)]
pub struct PodMetrics {
    pub pod_name: String,
    pub namespace: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub error_rate: f64,
    pub latency_ms: f64,
}

impl PodMetrics {
    pub fn exceeds_threshold(
        &self,
        cpu_threshold: f64,
        memory_threshold: f64,
        error_rate_threshold: f64,
        latency_threshold: f64,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        if self.cpu_usage > cpu_threshold {
            violations.push(format!("CPU usage {} > {}", self.cpu_usage, cpu_threshold));
        }
        if self.memory_usage > memory_threshold {
            violations.push(format!(
                "Memory usage {} > {}",
                self.memory_usage, memory_threshold
            ));
        }
        if self.error_rate > error_rate_threshold {
            violations.push(format!(
                "Error rate {} > {}",
                self.error_rate, error_rate_threshold
            ));
        }
        if self.latency_ms > latency_threshold {
            violations.push(format!(
                "Latency {}ms > {}ms",
                self.latency_ms, latency_threshold
            ));
        }

        violations
    }
}
