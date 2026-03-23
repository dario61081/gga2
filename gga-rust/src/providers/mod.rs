//! AI Provider implementations
//! 
//! This module provides a trait-based abstraction for AI providers (SOLID: Dependency Inversion).
//! Each provider implements the `Provider` trait, making it easy to add new providers
//! without modifying existing code (Open/Closed principle).

use anyhow::{Context, Result};
use async_trait::async_trait;
use colored::Colorize;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

mod claude;
mod github;
mod lmstudio;
mod ollama;

/// Trait for AI providers (SOLID: Interface Segregation)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name for display
    fn name(&self) -> &str;
    
    /// Execute a review with the given prompt
    async fn execute(&self, prompt: &str) -> Result<String>;
    
    /// Validate that the provider is available (CLI installed, API accessible, etc.)
    async fn validate(&self) -> Result<()>;
}

/// Provider factory (SOLID: Single Responsibility)
pub struct ProviderFactory;

impl ProviderFactory {
    /// Parse a provider string and return the appropriate provider
    pub fn create(provider_str: &str) -> Result<Box<dyn Provider>> {
        let (base_provider, model) = Self::parse_provider_string(provider_str);
        
        match base_provider {
            "claude" => Ok(Box::new(claude::ClaudeProvider::new())),
            "gemini" => {
                warn!("Gemini provider not yet implemented in Rust version");
                anyhow::bail!("Gemini provider coming soon")
            }
            "codex" => {
                warn!("Codex provider not yet implemented in Rust version");
                anyhow::bail!("Codex provider coming soon")
            }
            "opencode" => {
                warn!("OpenCode provider not yet implemented in Rust version");
                anyhow::bail!("OpenCode provider coming soon")
            }
            "ollama" => {
                let model = model.context("Ollama requiere un modelo (ej: ollama:llama3.2)")?;
                Ok(Box::new(ollama::OllamaProvider::new(model)))
            }
            "lmstudio" => Ok(Box::new(lmstudio::LMStudioProvider::new(model))),
            "github" => {
                let model = model.context("GitHub Models requiere un modelo (ej: github:gpt-4o)")?;
                Ok(Box::new(github::GitHubModelsProvider::new(model)))
            }
            _ => anyhow::bail!("Proveedor desconocido: {}", base_provider),
        }
    }
    
    /// Parse "provider:model" string into base and model parts
    fn parse_provider_string(s: &str) -> (&str, Option<String>) {
        if let Some((base, model)) = s.split_once(':') {
            (base, Some(model.to_string()))
        } else {
            (s, None)
        }
    }
}

/// Ejecutar proveedor con timeout y feedback de progreso (KISS: Simple wrapper)
pub async fn execute_with_timeout(
    provider: &dyn Provider,
    prompt: &str,
    timeout_secs: u64,
) -> Result<String> {
    let duration = Duration::from_secs(timeout_secs);
    let provider_name = provider.name().to_string();
    
    info!("Enviando a {} para revisión (timeout: {}s)", provider_name, timeout_secs);
    
    // Crear spinner de progreso (UX: Mejor experiencia de usuario)
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message(format!("Esperando respuesta de {}...", provider_name));
    spinner.enable_steady_tick(Duration::from_millis(100));
    
    // Ejecutar con timeout
    let result = timeout(duration, provider.execute(prompt)).await;
    
    spinner.finish_and_clear();
    
    match result {
        Ok(Ok(response)) => {
            debug!("Proveedor {} respondió exitosamente", provider_name);
            Ok(response)
        }
        Ok(Err(e)) => {
            warn!("Proveedor {} falló: {}", provider_name, e);
            Err(e)
        }
        Err(_) => {
            warn!("Proveedor {} excedió el timeout de {}s", provider_name, timeout_secs);
            anyhow::bail!(
                "{}\n\nEl proveedor excedió el timeout de {}s.\nIntenta: Aumentar TIMEOUT en la configuración .gga",
                "TIMEOUT".red().bold(),
                timeout_secs
            )
        }
    }
}

/// Validar una cadena de proveedor
pub async fn validate_provider(provider_str: &str) -> Result<()> {
    let provider = ProviderFactory::create(provider_str)?;
    provider.validate().await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_provider_string() {
        assert_eq!(
            ProviderFactory::parse_provider_string("claude"),
            ("claude", None)
        );
        assert_eq!(
            ProviderFactory::parse_provider_string("ollama:llama3.2"),
            ("ollama", Some("llama3.2".to_string()))
        );
        assert_eq!(
            ProviderFactory::parse_provider_string("github:gpt-4o"),
            ("github", Some("gpt-4o".to_string()))
        );
    }
}