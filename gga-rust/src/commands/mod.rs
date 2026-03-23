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

/// Ejecutar el comando uninstall
pub fn execute_uninstall() -> Result<()> {
    use crate::git::GitOperations;
    use crate::utils::output;
    
    output::print_banner(crate::utils::defaults::VERSION);
    
    // Verificar que estamos en un repo git
    if git2::Repository::open_from_env().is_err() {
        output::log_error("No es un repositorio git");
        std::process::exit(1);
    }
    
    // Desinstalar hook pre-commit
    match GitOperations::uninstall_hook("pre-commit") {
        Ok(_) => output::log_success("Hook pre-commit desinstalado"),
        Err(e) => output::log_warning(format!("Error al desinstalar pre-commit: {}", e)),
    }
    
    // Desinstalar hook commit-msg
    match GitOperations::uninstall_hook("commit-msg") {
        Ok(_) => output::log_success("Hook commit-msg desinstalado"),
        Err(e) => output::log_warning(format!("Error al desinstalar commit-msg: {}", e)),
    }
    
    println!();
    Ok(())
}