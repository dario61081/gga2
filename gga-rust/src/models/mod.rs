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
    about = "Gentleman Guardian Angel - Revisión de código usando IA",
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
    /// Ejecutar revisión de código en archivos staged
    Run(RunArgs),

    /// Instalar hooks de git
    Install(InstallArgs),

    /// Desinstalar hooks de git
    Uninstall,

    /// Mostrar configuración actual
    Config,

    /// Crear archivo de configuración .gga de ejemplo
    Init,

    /// Gestionar caché
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

/// Cache subcommands
#[derive(Subcommand)]
pub enum CacheAction {
    /// Mostrar estado del caché
    Status,

    /// Limpiar caché del proyecto
    Clear,

    /// Limpiar todo el caché
    ClearAll,
}

/// Arguments for the `run` command
#[derive(Args, Clone)]
pub struct RunArgs {
    /// Forzar revisión de todos los archivos, ignorando caché
    #[arg(long)]
    pub no_cache: bool,

    /// Modo CI: revisar archivos cambiados en el último commit
    #[arg(long)]
    pub ci: bool,

    /// Modo PR: revisar todos los archivos cambiados en el PR
    #[arg(long = "pr-mode")]
    pub pr_mode: bool,

    /// Con --pr-mode: enviar solo diffs (más rápido, más barato)
    #[arg(long = "diff-only")]
    pub diff_only: bool,
}

/// Arguments for the `install` command
#[derive(Args, Clone)]
pub struct InstallArgs {
    /// Instalar hook commit-msg en lugar de pre-commit
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
