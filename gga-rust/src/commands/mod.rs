//! Command implementations
//! 
//! Each command is a separate module following SOC principles.

mod run;
mod install;
mod config;
mod init;
mod cache;

pub use run::execute_run;
pub use install::execute_install;
pub use config::execute_config;
pub use init::execute_init;
pub use cache::execute_cache;

use anyhow::Result;

/// Execute the uninstall command
pub fn execute_uninstall() -> Result<()> {
    use crate::git::GitOperations;
    use crate::utils::output;
    
    output::print_banner(crate::utils::defaults::VERSION);
    
    // Check if we're in a git repo
    if git2::Repository::open_from_env().is_err() {
        output::log_error("Not a git repository");
        std::process::exit(1);
    }
    
    // Uninstall pre-commit hook
    match GitOperations::uninstall_hook("pre-commit") {
        Ok(_) => output::log_success("Uninstalled pre-commit hook"),
        Err(e) => output::log_warning(format!("Failed to uninstall pre-commit: {}", e)),
    }
    
    // Uninstall commit-msg hook
    match GitOperations::uninstall_hook("commit-msg") {
        Ok(_) => output::log_success("Uninstalled commit-msg hook"),
        Err(e) => output::log_warning(format!("Failed to uninstall commit-msg: {}", e)),
    }
    
    println!();
    Ok(())
}