//! Config command implementation

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::ConfigLoader;
use crate::utils::output;

/// Execute the config command
pub fn execute_config() -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    let config = ConfigLoader::load().context("Failed to load configuration")?;

    // Check config sources
    let global_path = std::env::var("APPDATA")
        .map(|p| format!("{}/gga/config", p))
        .or_else(|_| std::env::var("XDG_CONFIG_HOME").map(|p| format!("{}/gga/config", p)))
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .map(|p| format!("{}/gga/config", p.display()))
                .unwrap_or_default()
        });

    let project_path = ".gga";

    output::log_section("Config Files:");

    if std::path::Path::new(&global_path).exists() {
        println!("  Global:  {}", global_path.green());
    } else {
        println!("  Global:  {}", "Not found".yellow());
    }

    if std::path::Path::new(project_path).exists() {
        println!("  Project: {}", project_path.green());
    } else {
        println!("  Project: {}", "Not found".yellow());
    }
    println!();

    output::log_section("Values:");

    match &config.provider {
        Some(p) => println!("  PROVIDER:          {}", p.green()),
        None => println!("  PROVIDER:          {}", "Not configured".red()),
    }

    println!("  FILE_PATTERNS:     {}", config.file_patterns.cyan());

    if config.exclude_patterns.is_empty() {
        println!("  EXCLUDE_PATTERNS:  {}", "None".yellow());
    } else {
        println!("  EXCLUDE_PATTERNS:  {}", config.exclude_patterns.cyan());
    }

    println!("  RULES_FILE:        {}", config.rules_file.cyan());
    println!(
        "  STRICT_MODE:       {}",
        config.strict_mode.to_string().cyan()
    );
    println!(
        "  TIMEOUT:           {}s",
        config.timeout.to_string().cyan()
    );

    match &config.pr_base_branch {
        Some(b) => println!("  PR_BASE_BRANCH:    {}", b.cyan()),
        None => println!("  PR_BASE_BRANCH:    {}", "auto-detect".yellow()),
    }

    println!();

    output::log_section("Rules File:");

    if std::path::Path::new(&config.rules_file).exists() {
        println!("  {}", "Found".green());
    } else {
        println!("  {} ({})", "Not found".red(), config.rules_file);
    }

    println!();

    Ok(())
}
