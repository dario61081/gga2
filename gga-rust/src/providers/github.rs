//! GitHub Models provider implementation
//! 
//! Uses GitHub Models API (OpenAI-compatible) for code review.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::Provider;

/// GitHub Models API endpoint
const GITHUB_MODELS_ENDPOINT: &str = "https://models.inference.ai.azure.com/chat/completions";

/// GitHub Models provider (SOLID: Single Responsibility)
pub struct GitHubModelsProvider {
    model: String,
    client: Client,
}

/// Request body for GitHub Models API (OpenAI-compatible)
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Response from GitHub Models API
#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    error: Option<GitHubError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct GitHubError {
    message: String,
}

impl GitHubModelsProvider {
    /// Create a new GitHub Models provider
    pub fn new(model: String) -> Self {
        Self {
            model,
            client: Client::new(),
        }
    }
    
    /// Get GitHub auth token from gh CLI
    async fn get_auth_token(&self) -> Result<String> {
        let output = tokio::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .await
            .context("gh CLI not found. Install from https://cli.github.com")?;
        
        if !output.status.success() {
            anyhow::bail!(
                "GitHub authentication failed. Run 'gh auth login' first."
            );
        }
        
        let token = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        if token.is_empty() {
            anyhow::bail!("Empty auth token. Run 'gh auth login' to authenticate.");
        }
        
        Ok(token)
    }
}

#[async_trait]
impl Provider for GitHubModelsProvider {
    fn name(&self) -> &str {
        "GitHub Models"
    }
    
    async fn execute(&self, prompt: &str) -> Result<String> {
        info!("Executing GitHub Models API with model: {}", self.model);
        
        let token = self.get_auth_token().await?;
        
        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful code review assistant.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.2,
        };
        
        debug!("Sending request to GitHub Models API");
        
        let response = self.client
            .post(GITHUB_MODELS_ENDPOINT)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to connect to GitHub Models API")?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub Models API returned error {}: {}", status, error_text);
        }
        
        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse GitHub Models response")?;
        
        if let Some(error) = chat_response.error {
            anyhow::bail!("GitHub Models error: {}", error.message);
        }
        
        let content = chat_response
            .choices
            .and_then(|mut c| c.pop())
            .map(|c| c.message.content)
            .context("GitHub Models returned empty response")?;
        
        Ok(content)
    }
    
    async fn validate(&self) -> Result<()> {
        debug!("Validating GitHub Models configuration");
        
        // Check gh CLI is installed and authenticated
        let token = self.get_auth_token().await?;
        
        // Test API access with a minimal request
        let test_request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: 0.0,
        };
        
        let response = self.client
            .post(GITHUB_MODELS_ENDPOINT)
            .header("Authorization", format!("Bearer {}", token))
            .json(&test_request)
            .send()
            .await
            .context("Cannot connect to GitHub Models API")?;
        
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("GitHub authentication expired. Run 'gh auth login' again.");
        }
        
        info!("GitHub Models validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_provider() {
        let provider = GitHubModelsProvider::new("gpt-4o".to_string());
        assert_eq!(provider.name(), "GitHub Models");
    }
}