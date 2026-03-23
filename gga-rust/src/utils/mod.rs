//! Utility functions and helpers

use colored::Colorize;
use std::fmt::Display;

/// Logging and output utilities (SOC: Output concerns separated)
pub mod output {
    use colored::Colorize;
    
    /// Print a banner header
    pub fn print_banner(version: &str) {
        println!();
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
        println!("{} {}", "  Gentleman Guardian Angel".cyan().bold(), format!("v{}", version).cyan());
        println!("{}", "  Revisión de código usando IA".cyan());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
        println!();
    }
    
    /// Log info message
    pub fn log_info(msg: impl std::fmt::Display) {
        println!("{} {}", "ℹ️".blue(), msg);
    }
    
    /// Log success message
    pub fn log_success(msg: impl std::fmt::Display) {
        println!("{} {}", "✅".green(), msg.to_string().green());
    }
    
    /// Log warning message
    pub fn log_warning(msg: impl std::fmt::Display) {
        eprintln!("{} {}", "⚠️".yellow(), msg.to_string().yellow());
    }
    
    /// Log error message
    pub fn log_error(msg: impl std::fmt::Display) {
        eprintln!("{} {}", "❌".red(), msg.to_string().red());
    }
    
    /// Log section header
    pub fn log_section(title: impl std::fmt::Display) {
        println!();
        println!("{}", title.to_string().bold());
    }
}

/// Configuration defaults (KISS: Constants in one place)
pub mod defaults {
    pub const VERSION: &str = "3.0.0";
    pub const FILE_PATTERNS: &str = "*";
    pub const RULES_FILE: &str = "AGENTS.md";
    pub const STRICT_MODE: bool = true;
    pub const TIMEOUT: u64 = 300;
    pub const UPDATE_CHECK_REPO: &str = "Gentleman-Programming/gentleman-guardian-angel";
}

/// Pattern matching utilities (SOC: Matching concerns separated)
pub mod patterns {
    use glob::Pattern;
    use std::path::Path;
    
    /// Check if file matches comma-separated patterns
    pub fn matches_any(path: &Path, patterns: &str) -> bool {
        if patterns.is_empty() || patterns == "*" {
            return true;
        }
        
        let patterns: Vec<Pattern> = patterns
            .split(',')
            .filter_map(|p| Pattern::new(p.trim()).ok())
            .collect();
        
        let path_str = path.to_string_lossy();
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        patterns.iter().any(|p| {
            p.matches(&path_str) || p.matches(&filename)
        })
    }
}

/// Version checking utilities
pub mod version {
    use super::defaults;
    use crate::utils::output;
    
    /// Check for newer version on GitHub
    pub async fn check_for_update(current: &str) {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            defaults::UPDATE_CHECK_REPO
        );
        
        let client = reqwest::Client::new();
        let response = match client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gga-cli")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return, // Silent on network errors
        };
        
        if !response.status().is_success() {
            return;
        }
        
        let json: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(_) => return,
        };
        
        let latest_tag = json["tag_name"]
            .as_str()
            .and_then(|t| t.strip_prefix('v'));
        
        if let Some(latest) = latest_tag {
            if latest != current {
                output::log_warning(format!(
                    "Update available: v{} → v{}",
                    current, latest
                ));
            }
        }
    }
}

/// File utilities
pub mod files {
    use anyhow::{Context, Result};
    use std::path::Path;
    
    /// Read file content from either git staging or filesystem
    pub fn read_file(path: &Path, use_staged: bool) -> Result<String> {
        if use_staged {
            crate::git::GitOperations::read_staged_file(path)
        } else {
            std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {:?}", path))
        }
    }
    
    /// Compute file hash
    pub fn compute_hash(path: &Path) -> Result<String> {
        crate::cache::CacheManager::hash_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_matches_any() {
        assert!(patterns::matches_any(Path::new("src/main.rs"), "*.rs"));
        assert!(patterns::matches_any(Path::new("src/main.ts"), "*.ts,*.tsx"));
        assert!(!patterns::matches_any(Path::new("test.ts"), "*.test.ts"));
    }
}