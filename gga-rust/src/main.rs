//! Gentleman Guardian Angel - Provider-agnostic code review using AI
//! 
//! A standalone CLI tool that validates staged files against your project's
//! coding standards using any AI provider (Claude, Gemini, Codex, Ollama, etc.)
//! 
//! This Rust implementation follows SOLID, KISS, and SOC principles:
//! - SOLID: Each module has a single responsibility
//! - KISS: Simple, straightforward implementations
//! - SOC: Clear separation between CLI, config, providers, cache, and git

mod commands;
mod config;
mod models;
mod providers;
mod cache;
mod git;
mod utils;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::models::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for debug logging
    init_tracing();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Dispatch to appropriate command
    match cli.command {
        models::Commands::Run(args) => {
            commands::execute_run(&args).await?;
        }
        models::Commands::Install(args) => {
            commands::execute_install(&args)?;
        }
        models::Commands::Uninstall => {
            commands::execute_uninstall()?;
        }
        models::Commands::Config => {
            commands::execute_config()?;
        }
        models::Commands::Init => {
            commands::execute_init()?;
        }
        models::Commands::Cache { action } => {
            commands::execute_cache(&action)?;
        }
    }
    
    Ok(())
}

/// Initialize tracing subscriber for debug logging
fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "gga=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
}