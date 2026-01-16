mod claude;
mod gemini;
mod ollama;
mod openai;
mod traits;

pub use claude::ClaudeClient;
pub use gemini::GeminiClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;
pub use traits::*;

use crate::crd::LlmProvider;
use crate::error::Result;

pub async fn create_llm_client(
    provider: &LlmProvider,
    api_key: &str,
    model: &str,
    base_url: Option<&str>,
    timeout_seconds: u64,
) -> Result<Box<dyn LlmClient>> {
    match provider {
        LlmProvider::Claude => {
            let client = ClaudeClient::new(api_key, model, timeout_seconds)?;
            Ok(Box::new(client))
        }
        LlmProvider::OpenAI => {
            let client = OpenAIClient::new(api_key, model, base_url, timeout_seconds)?;
            Ok(Box::new(client))
        }
        LlmProvider::Gemini => {
            let client = GeminiClient::new(api_key, model, timeout_seconds)?;
            Ok(Box::new(client))
        }
        LlmProvider::Ollama => {
            let url = base_url.unwrap_or("http://localhost:11434");
            let client = OllamaClient::new(url, model, timeout_seconds)?;
            Ok(Box::new(client))
        }
    }
}
