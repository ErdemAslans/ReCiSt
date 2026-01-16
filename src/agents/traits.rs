use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::models::{AgentEvent, AgentType};

#[async_trait]
pub trait Agent: Send + Sync {
    fn agent_type(&self) -> AgentType;

    async fn start(&self) -> Result<()>;

    async fn stop(&self) -> Result<()>;

    fn subscribe_to(&self) -> Vec<crate::models::AgentEventType>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle_event(&self, event: AgentEvent) -> Result<Option<AgentEvent>>;
}
