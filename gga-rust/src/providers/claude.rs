//! Claude provider implementation
//! 
//! Uses the Claude Code CLI for code review.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use super::Provider;

/// Claude provider (SOLID: Single Responsibility)
pub struct ClaudeProvider {
    #[allow(dead_code)]
    cli_path: String,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    pub fn new() -> Self {
        Self {
            cli_path: "claude".to_string(),
        }
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude"
    }
    
    async fn execute(&self, prompt: &str) -> Result<String> {
        info!("Executing Claude CLI");
        
        // Spawn Claude CLI process
        let mut child = tokio::process::Command::new(&self.cli_path)
            .arg("--print")
            .arg("--output-format=text")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn Claude CLI. Is it installed?")?;
        
        // Write prompt to stdin
        if let Some(stdin) = child.stdin.take() {
            let mut stdin = stdin;
            stdin.write_all(prompt.as_bytes()).await
                .context("Failed to write to Claude stdin")?;
            // Drop stdin to signal EOF
            drop(stdin);
        }
        
        // Wait for output
        let output = child.wait_with_output()
            .await
            .context("Failed to read Claude output")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Claude CLI failed: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
    
    async fn validate(&self) -> Result<()> {
        debug!("Validating Claude CLI");
        
        let output = tokio::process::Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .await
            .context("Claude CLI not found. Install from https://claude.ai/code")?;
        
        if !output.status.success() {
            anyhow::bail!("Claude CLI validation failed");
        }
        
        let version = String::from_utf8_lossy(&output.stdout);
        info!("Claude CLI validated: {}", version.trim());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_provider() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.name(), "Claude");
    }
}