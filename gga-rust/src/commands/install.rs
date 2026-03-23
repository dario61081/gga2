//! Install command implementation

use anyhow::{Context, Result};

use crate::git::GitOperations;
use crate::models::InstallArgs;
use crate::utils::output;

/// Execute the install command
pub fn execute_install(args: &InstallArgs) -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    // Check if we're in a git repo
    if git2::Repository::open_from_env().is_err() {
        output::log_error("Not a git repository");
        std::process::exit(1);
    }

    let hook_type = if args.commit_msg {
        "commit-msg"
    } else {
        "pre-commit"
    };

    GitOperations::install_hook(hook_type, args.commit_msg)?;

    output::log_success(format!("Installed {} hook", hook_type));
    println!();

    Ok(())
}
