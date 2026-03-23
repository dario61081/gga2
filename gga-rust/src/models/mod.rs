//! Data models for GGA
//!
//! This module contains pure data structures without business logic (KISS principle).

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main CLI structure (SOC: CLI concerns separated)
#[derive(Parser)]
#[command(
    name = "gga",
    version,
    about = "Gentleman Guardian Angel - Provider-agnostic code review using AI",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands (SOLID: Open/Closed principle - easy to extend)
#[derive(Subcommand)]
pub enum Commands {
    /// Run code review on staged files
    Run(RunArgs),

    /// Install git hooks
    Install(InstallArgs),

    /// Uninstall git hooks
    Uninstall,

    /// Show current configuration
    Config,

    /// Create a sample .gga config file
    Init,

    /// Manage cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

/// Cache subcommands
#[derive(Subcommand)]
pub enum CacheAction {
    /// Show cache status
    Status,

    /// Clear project cache
    Clear,

    /// Clear all cached data
    ClearAll,
}

/// Arguments for the `run` command
#[derive(Args, Clone)]
pub struct RunArgs {
    /// Force review all files, ignoring cache
    #[arg(long)]
    pub no_cache: bool,

    /// CI mode: review files changed in last commit
    #[arg(long)]
    pub ci: bool,

    /// PR mode: review all files changed in the PR
    #[arg(long = "pr-mode")]
    pub pr_mode: bool,

    /// With --pr-mode: send only diffs (faster, cheaper)
    #[arg(long = "diff-only")]
    pub diff_only: bool,
}

/// Arguments for the `install` command
#[derive(Args, Clone)]
pub struct InstallArgs {
    /// Install commit-msg hook instead of pre-commit
    #[arg(long = "commit-msg")]
    pub commit_msg: bool,
}

/// Configuration file structure (KISS: Simple, flat structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// AI provider to use (required)
    pub provider: Option<String>,

    /// File patterns to include in review
    #[serde(default = "default_file_patterns")]
    pub file_patterns: String,

    /// File patterns to exclude from review
    #[serde(default)]
    pub exclude_patterns: String,

    /// File containing review rules
    #[serde(default = "default_rules_file")]
    pub rules_file: String,

    /// Fail on ambiguous AI response
    #[serde(default = "default_strict_mode")]
    pub strict_mode: bool,

    /// Max seconds to wait for AI response
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Base branch for PR mode
    pub pr_base_branch: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: None,
            file_patterns: default_file_patterns(),
            exclude_patterns: String::new(),
            rules_file: default_rules_file(),
            strict_mode: default_strict_mode(),
            timeout: default_timeout(),
            pr_base_branch: None,
        }
    }
}

fn default_file_patterns() -> String {
    "*".to_string()
}

fn default_rules_file() -> String {
    "AGENTS.md".to_string()
}

fn default_strict_mode() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

/// Provider type enum (SOLID: Single Responsibility - only provider identification)
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Claude,
    Gemini,
    Codex,
    OpenCode(Option<String>),
    Ollama(String),
    LMStudio(Option<String>),
    GitHubModels(String),
}

/// Review result (KISS: Simple result enum)
#[derive(Debug, Clone)]
pub enum ReviewStatus {
    Passed,
    Failed(String),
    Ambiguous,
}

/// File to review (SOC: Data only, no logic)
#[derive(Debug, Clone)]
pub struct FileToReview {
    pub path: PathBuf,
    pub content: String,
    pub hash: String,
}

/// Review context (SOC: Grouped related data)
#[derive(Debug, Clone)]
pub struct ReviewContext {
    pub rules: String,
    pub files: Vec<FileToReview>,
    pub commit_message: Option<String>,
}
