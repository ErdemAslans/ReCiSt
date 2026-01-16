# ReCiSt: Bio-inspired Agentic Self-Healing Framework

ReCiSt is a Kubernetes operator that provides automatic self-healing capabilities inspired by the human body's wound healing process.

## Architecture

The system implements four biological phases:

| Phase | Biological | System Component | Target Time |
|-------|------------|------------------|-------------|
| 1 | Hemostasis (Stop bleeding) | Containment Agent | 5s |
| 2 | Inflammation | Diagnosis Agent | 15s |
| 3 | Proliferation | MetaCognitive Agent | 20s |
| 4 | Remodeling | Knowledge Agent | 5s |

## Components

### Agents

- **ContainmentAgent**: Detects faults via Prometheus metrics, isolates faulty pods using NetworkPolicy
- **DiagnosisAgent**: Collects logs from Loki, analyzes with LLM to find root cause
- **MetaCognitiveAgent**: Generates solution strategies using parallel micro-agents, executes healing actions
- **KnowledgeAgent**: Records healing events in Qdrant vector database, enables learning from past incidents

### Clients

- Prometheus client for metrics
- Loki client for logs
- LLM client (Claude, OpenAI, Gemini, Ollama)
- Qdrant client for vector storage
- Redis client for local caching

## Installation

### Prerequisites

- Kubernetes cluster (1.26+)
- Prometheus and Loki for observability
- Qdrant for vector database
- Redis for caching
- LLM API key (Claude, OpenAI, Gemini, or local Ollama)

### Using Helm

```bash
# Create namespace
kubectl create namespace recist-system

# Create LLM API key secret
kubectl create secret generic llm-api-key \
  --namespace recist-system \
  --from-literal=key=your-api-key

# Install ReCiSt
helm install recist ./helm/recist --namespace recist-system
```

### Building from Source

```bash
# Build
cargo build --release

# Build Docker image
docker build -t recist:latest .
```

## Configuration

Create a `SelfHealingPolicy` to define monitoring targets and thresholds:

```yaml
apiVersion: recist.io/v1alpha1
kind: SelfHealingPolicy
metadata:
  name: default-policy
spec:
  targetNamespaces:
    - default
  thresholds:
    cpu: 0.9
    memory: 0.85
    latencyMs: 500
    errorRate: 0.05
  allowedActions:
    - restart
    - scale
  llmConfig:
    provider: claude
    model: claude-3-sonnet-20240229
    apiKeySecret: llm-api-key
```

## Custom Resource Definitions

### SelfHealingPolicy

Defines what to monitor and how to heal.

### HealingEvent

Records each healing attempt with diagnosis, actions, and outcome.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PROMETHEUS_URL` | Prometheus server URL | `http://prometheus:9090` |
| `LOKI_URL` | Loki server URL | `http://loki:3100` |
| `QDRANT_URL` | Qdrant server URL | `http://qdrant:6334` |
| `REDIS_URL` | Redis server URL | `redis://redis:6379` |
| `LLM_API_KEY` | LLM API key | - |
| `RUST_LOG` | Log level | `info` |

## Performance Targets

| Metric | Target |
|--------|--------|
| Total healing time | < 60s |
| Correct detection rate | > 95% |
| Successful healing rate | > 90% |
| Auto-resolution rate | > 80% |

## License

Apache-2.0

## References

- Paper: "Bio-inspired Agentic Self-healing Framework for Resilient Distributed Computing Continuum Systems"
- arXiv: 2601.00339
