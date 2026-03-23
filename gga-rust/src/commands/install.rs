//! Install command implementation

use anyhow::{Context, Result};

use crate::git::GitOperations;
use crate::models::InstallArgs;
use crate::utils::output;

/// Ejecutar el comando install
pub fn execute_install(args: &InstallArgs) -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    // Verificar que estamos en un repo git
    if git2::Repository::open_from_env().is_err() {
        output::log_error("No es un repositorio git");
        std::process::exit(1);
    }

    let hook_type = if args.commit_msg {
        "commit-msg"
    } else {
        "pre-commit"
    };

    GitOperations::install_hook(hook_type, args.commit_msg)?;

    output::log_success(format!("Hook {} instalado", hook_type));
    println!();

    Ok(())
}
