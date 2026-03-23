//! Init command implementation

use anyhow::{Context, Result};

use crate::config::ConfigLoader;
use crate::utils::output;

/// Execute the init command
pub fn execute_init() -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    let config_path = std::path::Path::new(".gga");

    if config_path.exists() {
        output::log_warning("Config file already exists: .gga");

        // In a real implementation, we'd prompt for confirmation
        // For now, just inform the user
        output::log_info("Overwrite? Delete .gga and run 'gga init' again to recreate.");
        println!();
        return Ok(());
    }

    ConfigLoader::create_sample_config()?;

    output::log_success("Created config file: .gga");
    println!();

    output::log_info("Next steps:");
    println!("  1. Edit .gga to set your preferred provider");
    println!("  2. Create AGENTS.md with your coding standards");
    println!("  3. Run: gga install");
    println!();

    Ok(())
}
