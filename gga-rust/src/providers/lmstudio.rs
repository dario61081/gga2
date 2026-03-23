//! LM Studio provider implementation
//! 
//! Uses LM Studio's local API server (OpenAI-compatible).

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::Provider;

/// Default LM Studio endpoint
const DEFAULT_ENDPOINT: &str = "http://localhost:1234/v1/chat/completions";

/// LM Studio provider (SOLID: Single Responsibility)
pub struct LMStudioProvider {
    model: String,
    endpoint: String,
    client: Client,
}

/// Request body (OpenAI-compatible)
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Response from LM Studio
#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    error: Option<LMStudioError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct LMStudioError {
    message: Option<String>,
}

impl LMStudioProvider {
    /// Create a new LM Studio provider
    pub fn new(model: Option<String>) -> Self {
        let endpoint = std::env::var("LMSTUDIO_HOST")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
        
        // Ensure endpoint ends with /v1/chat/completions
        let endpoint = if endpoint.ends_with("/chat/completions") {
            endpoint
        } else if endpoint.ends_with("/v1") {
            format!("{}/chat/completions", endpoint)
        } else if endpoint.ends_with('/') {
            format!("{}v1/chat/completions", endpoint)
        } else {
            format!("{}/v1/chat/completions", endpoint)
        };
        
        Self {
            model: model.unwrap_or_else(|| "local-model".to_string()),
            endpoint,
            client: Client::new(),
        }
    }
    
    /// Validate endpoint format (security)
    fn validate_endpoint(endpoint: &str) -> Result<()> {
        let url = url::Url::parse(endpoint)
            .context("Invalid LMSTUDIO_HOST format")?;
        
        if url.scheme() != "http" && url.scheme() != "https" {
            anyhow::bail!("LMSTUDIO_HOST must use http or https scheme");
        }
        
        Ok(())
    }
}

#[async_trait]
impl Provider for LMStudioProvider {
    fn name(&self) -> &str {
        "LM Studio"
    }
    
    async fn execute(&self, prompt: &str) -> Result<String> {
        info!("Executing LM Studio API at {}", self.endpoint);
        
        Self::validate_endpoint(&self.endpoint)?;
        
        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
            stream: false,
        };
        
        debug!("Sending request to LM Studio");
        
        let response = self.client
            .post(&self.endpoint)
            .json(&request_body)
            .send()
            .await
            .context("Failed to connect to LM Studio. Is the server running?")?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LM Studio returned error {}: {}", status, error_text);
        }
        
        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse LM Studio response")?;
        
        if let Some(error) = chat_response.error {
            let msg = error.message.unwrap_or_else(|| "Unknown error".to_string());
            anyhow::bail!("LM Studio error: {}", msg);
        }
        
        let content = chat_response
            .choices
            .and_then(|mut c| c.pop())
            .map(|c| c.message.content)
            .context("LM Studio returned empty response")?;
        
        Ok(content)
    }
    
    async fn validate(&self) -> Result<()> {
        debug!("Validating LM Studio configuration");
        
        Self::validate_endpoint(&self.endpoint)?;
        
        // Try to connect to check if server is running
        let response = self.client
            .get(self.endpoint.replace("/chat/completions", "/models"))
            .send()
            .await;
        
        match response {
            Ok(resp) if resp.status().is_success() => {
                info!("LM Studio validation passed");
                Ok(())
            }
            Ok(resp) => {
                anyhow::bail!("LM Studio returned status: {}", resp.status());
            }
            Err(_) => {
                anyhow::bail!(
                    "Cannot connect to LM Studio at {}. Is the server running?",
                    self.endpoint
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_provider() {
        let provider = LMStudioProvider::new(None);
        assert_eq!(provider.name(), "LM Studio");
        assert!(provider.endpoint.contains("1234"));
    }
    
    #[test]
    fn test_validate_endpoint() {
        assert!(LMStudioProvider::validate_endpoint("http://localhost:1234/v1").is_ok());
        assert!(LMStudioProvider::validate_endpoint("https://lmstudio.local").is_ok());
        assert!(LMStudioProvider::validate_endpoint("ftp://invalid").is_err());
    }
}