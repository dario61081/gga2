//! Ollama provider implementation
//! 
//! Supports local Ollama instances via REST API.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::Provider;

/// Ollama provider (SOLID: Single Responsibility - only Ollama concerns)
pub struct OllamaProvider {
    model: String,
    host: String,
    client: Client,
}

/// Request body for Ollama API
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Response from Ollama API
#[derive(Deserialize)]
struct OllamaResponse {
    response: Option<String>,
    error: Option<String>,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(model: String) -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        
        Self {
            model,
            host: host.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }
    
    /// Validate OLLAMA_HOST format (security: Prevent injection)
    fn validate_host(host: &str) -> Result<()> {
        // Block dangerous characters that could indicate command injection
        let dangerous_chars = [';', '|', '&', '`', '$', '(', ')', '<', '>'];
        if host.chars().any(|c| dangerous_chars.contains(&c)) {
            anyhow::bail!("OLLAMA_HOST contains invalid characters");
        }
        
        let url = url::Url::parse(host)
            .context("Invalid OLLAMA_HOST format")?;
        
        if url.scheme() != "http" && url.scheme() != "https" {
            anyhow::bail!("OLLAMA_HOST must use http or https scheme");
        }
        
        // Ensure host has a valid domain
        if url.host().is_none() {
            anyhow::bail!("OLLAMA_HOST must have a valid hostname");
        }
        
        Ok(())
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }
    
    async fn execute(&self, prompt: &str) -> Result<String> {
        // Validate host before making request (security)
        Self::validate_host(&self.host)?;
        
        let endpoint = format!("{}/api/generate", self.host);
        
        let request_body = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };
        
        debug!("Sending request to Ollama at {}", endpoint);
        
        let response = self.client
            .post(&endpoint)
            .json(&request_body)
            .send()
            .await
            .context("Failed to connect to Ollama")?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned error {}: {}", status, error_text);
        }
        
        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;
        
        if let Some(error) = ollama_response.error {
            anyhow::bail!("Ollama error: {}", error);
        }
        
        ollama_response
            .response
            .context("Ollama returned empty response")
    }
    
    async fn validate(&self) -> Result<()> {
        // Validate host format
        Self::validate_host(&self.host)?;
        
        // Check if Ollama is running
        let endpoint = format!("{}/api/tags", self.host);
        
        let response = self.client
            .get(&endpoint)
            .send()
            .await
            .context("Cannot connect to Ollama. Is it running?")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Ollama returned error: {}", response.status());
        }
        
        // Check if model is available
        let tags: serde_json::Value = response.json().await?;
        let models = tags["models"]
            .as_array()
            .context("Invalid response from Ollama")?;
        
        let model_available = models
            .iter()
            .any(|m| m["name"].as_str().map(|n| n.starts_with(&self.model)).unwrap_or(false));
        
        if !model_available {
            anyhow::bail!(
                "Model '{}' not found in Ollama. Available models: {:?}",
                self.model,
                models.iter()
                    .filter_map(|m| m["name"].as_str())
                    .collect::<Vec<_>>()
            );
        }
        
        info!("Ollama validation passed: model '{}' is available", self.model);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_host() {
        // Valid hosts
        assert!(OllamaProvider::validate_host("http://localhost:11434").is_ok());
        assert!(OllamaProvider::validate_host("https://ollama.example.com").is_ok());
        assert!(OllamaProvider::validate_host("http://192.168.1.100:11434").is_ok());
        
        // Invalid schemes
        assert!(OllamaProvider::validate_host("ftp://invalid").is_err());
        assert!(OllamaProvider::validate_host("ssh://localhost").is_err());
        
        // Command injection attempts (should be blocked)
        assert!(OllamaProvider::validate_host("http://localhost:11434/;rm -rf").is_err());
        assert!(OllamaProvider::validate_host("http://localhost:11434|cat /etc/passwd").is_err());
        assert!(OllamaProvider::validate_host("http://localhost:11434&whoami").is_err());
        assert!(OllamaProvider::validate_host("http://localhost:11434`id`").is_err());
        assert!(OllamaProvider::validate_host("http://localhost:11434$(whoami)").is_err());
        
        // Invalid URLs
        assert!(OllamaProvider::validate_host("not a url").is_err());
    }
}