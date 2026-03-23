//! Configuration loading and management
//!
//! This module handles reading configuration from multiple sources with proper precedence:
//! 1. Environment variables (highest priority)
//! 2. Project config (.gga)
//! 3. Global config (~/.config/gga/config or %APPDATA%/gga/config)
//! 4. Defaults (lowest priority)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::models::Config;

/// Configuration loader (SOLID: Single Responsibility)
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration with proper precedence
    pub fn load() -> Result<Config> {
        let mut config = Config::default();

        // Load global config first
        if let Some(global_config) = Self::load_global_config()? {
            debug!("Loaded global config from {:?}", global_config.path);
            config = Self::merge_configs(config, global_config.config);
        }

        // Load project config (overrides global)
        if let Some(project_config) = Self::load_project_config()? {
            debug!("Loaded project config from {:?}", project_config.path);
            config = Self::merge_configs(config, project_config.config);
        }

        // Apply environment variable overrides (highest priority)
        Self::apply_env_overrides(&mut config);

        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Get global config path based on OS (SOC: Platform concerns separated)
    fn get_global_config_path() -> Option<PathBuf> {
        // Windows: %APPDATA%/gga/config
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("gga").join("config"));
        }

        // XDG (Linux/Unix): $XDG_CONFIG_HOME/gga/config
        if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("gga").join("config"));
        }

        // Fallback: ~/.config/gga/config
        dirs::config_dir().map(|p| p.join("gga").join("config"))
    }

    /// Load global configuration file
    fn load_global_config() -> Result<Option<LoadedConfig>> {
        let path =
            Self::get_global_config_path().context("Failed to determine global config path")?;

        Self::load_config_file(&path)
    }

    /// Load project configuration file (.gga)
    fn load_project_config() -> Result<Option<LoadedConfig>> {
        let path = PathBuf::from(".gga");
        Self::load_config_file(&path)
    }

    /// Load and parse a config file (KISS: Single file loading logic)
    fn load_config_file(path: &Path) -> Result<Option<LoadedConfig>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        // Parse as TOML (supports comments via #)
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        Ok(Some(LoadedConfig {
            path: path.to_path_buf(),
            config,
        }))
    }

    /// Merge two configs, where `override_config` takes precedence
    fn merge_configs(mut base: Config, override_config: Config) -> Config {
        // Only override if the new value is set
        if override_config.provider.is_some() {
            base.provider = override_config.provider;
        }
        if override_config.file_patterns != "*" {
            base.file_patterns = override_config.file_patterns;
        }
        if !override_config.exclude_patterns.is_empty() {
            base.exclude_patterns = override_config.exclude_patterns;
        }
        if override_config.rules_file != "AGENTS.md" {
            base.rules_file = override_config.rules_file;
        }
        // Always override strict_mode and timeout if explicitly set
        base.strict_mode = override_config.strict_mode;
        base.timeout = override_config.timeout;
        if override_config.pr_base_branch.is_some() {
            base.pr_base_branch = override_config.pr_base_branch;
        }

        base
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(config: &mut Config) {
        // GGA_PROVIDER overrides provider
        if let Ok(provider) = std::env::var("GGA_PROVIDER") {
            debug!("Overriding provider from GGA_PROVIDER env var");
            config.provider = Some(provider);
        }

        // GGA_TIMEOUT overrides timeout
        if let Ok(timeout_str) = std::env::var("GGA_TIMEOUT") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                debug!("Overriding timeout from GGA_TIMEOUT env var: {}", timeout);
                config.timeout = timeout;
            }
        }
    }

    /// Create a sample config file
    pub fn create_sample_config() -> Result<()> {
        let sample = r#"# Gentleman Guardian Angel Configuration
# https://github.com/Gentleman-Programming/gentleman-guardian-angel

# AI Provider (required)
# Options: claude, gemini, codex, opencode, ollama:<model>, lmstudio[:model], github:<model>
# Examples:
#   provider = "claude"
#   provider = "ollama:llama3.2"
#   provider = "github:gpt-4o"
provider = "claude"

# File patterns to include in review (comma-separated)
# Default: "*" (all files)
# Example: "*.ts,*.tsx,*.js,*.jsx"
file_patterns = "*.ts,*.tsx,*.js,*.jsx"

# File patterns to exclude from review (comma-separated)
# Example: "*.test.ts,*.spec.ts"
exclude_patterns = "*.test.ts,*.spec.ts,*.d.ts"

# File containing code review rules
# Default: "AGENTS.md"
rules_file = "AGENTS.md"

# Strict mode: fail if AI response is ambiguous
# Default: true
strict_mode = true

# Timeout in seconds for AI provider response
# Default: 300 (5 minutes)
timeout = 300

# Base branch for --pr-mode (auto-detects main/master/develop if empty)
# pr_base_branch = "main"
"#;

        std::fs::write(".gga", sample).context("Failed to create .gga config file")?;

        info!("Created sample config file: .gga");
        Ok(())
    }
}

/// Internal struct for loaded config with metadata
struct LoadedConfig {
    #[allow(dead_code)]
    path: PathBuf,
    config: Config,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.provider.is_none());
        assert_eq!(config.file_patterns, "*");
        assert_eq!(config.rules_file, "AGENTS.md");
        assert!(config.strict_mode);
        assert_eq!(config.timeout, 300);
    }

    #[test]
    fn test_parse_config() -> Result<()> {
        let content = r#"
provider = "claude"
file_patterns = "*.rs"
timeout = 600
"#;
        let config: Config = toml::from_str(content)?;
        assert_eq!(config.provider, Some("claude".to_string()));
        assert_eq!(config.file_patterns, "*.rs");
        assert_eq!(config.timeout, 600);
        Ok(())
    }
}
