//! Init command implementation

use anyhow::{Context, Result};

use crate::config::ConfigLoader;
use crate::utils::output;

/// Ejecutar el comando init
pub fn execute_init() -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    let config_path = std::path::Path::new(".gga");

    if config_path.exists() {
        output::log_warning("El archivo de configuración ya existe: .gga");
        output::log_info(
            "¿Sobrescribir? Elimina .gga y ejecuta 'gga init' de nuevo para recrearlo.",
        );
        println!();
        return Ok(());
    }

    ConfigLoader::create_sample_config()?;

    output::log_success("Archivo de configuración creado: .gga");
    println!();

    output::log_info("Siguientes pasos:");
    println!("  1. Edita .gga para configurar tu proveedor preferido");
    println!("  2. Crea AGENTS.md con tus estándares de codificación");
    println!("  3. Ejecuta: gga install");
    println!();

    Ok(())
}
