use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::error::{RecistError, Result};
use crate::models::{AgentEvent, AgentEventType, AgentType};

const CHANNEL_CAPACITY: usize = 1024;

pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
    subscribers: Arc<RwLock<HashMap<AgentType, Vec<AgentEventType>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(
        &self,
        agent: AgentType,
        event_types: Vec<AgentEventType>,
    ) -> broadcast::Receiver<AgentEvent> {
        let mut subs = self.subscribers.write().await;
        subs.insert(agent.clone(), event_types);
        info!("Agent {:?} subscribed to event bus", agent);
        self.sender.subscribe()
    }

    pub async fn publish(&self, event: AgentEvent) -> Result<()> {
        debug!(
            "Publishing event {:?} from {:?}",
            event.event_type, event.source_agent
        );

        match self.sender.send(event.clone()) {
            Ok(receivers) => {
                debug!("Event sent to {} receivers", receivers);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish event: {}", e);
                Err(RecistError::EventBusError(format!(
                    "Failed to publish event: {}",
                    e
                )))
            }
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            subscribers: Arc::clone(&self.subscribers),
        }
    }
}

pub struct EventFilter {
    allowed_types: Vec<AgentEventType>,
}

impl EventFilter {
    pub fn new(allowed_types: Vec<AgentEventType>) -> Self {
        Self { allowed_types }
    }

    pub fn matches(&self, event: &AgentEvent) -> bool {
        self.allowed_types.is_empty() || self.allowed_types.contains(&event.event_type)
    }
}

pub struct FilteredReceiver {
    receiver: broadcast::Receiver<AgentEvent>,
    filter: EventFilter,
}

impl FilteredReceiver {
    pub fn new(receiver: broadcast::Receiver<AgentEvent>, filter: EventFilter) -> Self {
        Self { receiver, filter }
    }

    pub async fn recv(&mut self) -> Result<AgentEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Ok(event);
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(RecistError::EventBusError("Channel closed".to_string()));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Receiver lagged by {} messages", n);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventPayload, FaultCluster, FaultDetectedPayload};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();
        let mut receiver = bus
            .subscribe(AgentType::Diagnosis, vec![AgentEventType::FaultDetected])
            .await;

        let event =
            AgentEvent::fault_detected(Uuid::new_v4(), FaultCluster::new("default".to_string()));

        bus.publish(event.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, AgentEventType::FaultDetected);
    }

    #[tokio::test]
    async fn test_event_filter() {
        let filter = EventFilter::new(vec![AgentEventType::FaultDetected]);

        let fault_event =
            AgentEvent::fault_detected(Uuid::new_v4(), FaultCluster::new("default".to_string()));

        assert!(filter.matches(&fault_event));
    }
}
