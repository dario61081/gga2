//! Config command implementation

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::ConfigLoader;
use crate::utils::output;

/// Ejecutar el comando config
pub fn execute_config() -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    let config = ConfigLoader::load().context("Error al cargar la configuración")?;

    // Verificar fuentes de configuración
    let global_path = std::env::var("APPDATA")
        .map(|p| format!("{}/gga/config", p))
        .or_else(|_| std::env::var("XDG_CONFIG_HOME").map(|p| format!("{}/gga/config", p)))
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .map(|p| format!("{}/gga/config", p.display()))
                .unwrap_or_default()
        });

    let project_path = ".gga";

    output::log_section("Archivos de configuración:");

    if std::path::Path::new(&global_path).exists() {
        println!("  Global:  {}", global_path.green());
    } else {
        println!("  Global:  {}", "No encontrado".yellow());
    }

    if std::path::Path::new(project_path).exists() {
        println!("  Proyecto: {}", project_path.green());
    } else {
        println!("  Proyecto: {}", "No encontrado".yellow());
    }
    println!();

    output::log_section("Valores:");

    match &config.provider {
        Some(p) => println!("  PROVEEDOR:         {}", p.green()),
        None => println!("  PROVEEDOR:         {}", "No configurado".red()),
    }

    println!("  PATRONES_ARCHIVOS: {}", config.file_patterns.cyan());

    if config.exclude_patterns.is_empty() {
        println!("  PATRONES_EXCLUSION: {}", "Ninguno".yellow());
    } else {
        println!("  PATRONES_EXCLUSION: {}", config.exclude_patterns.cyan());
    }

    println!("  ARCHIVO_REGLAS:    {}", config.rules_file.cyan());
    println!(
        "  MODO_ESTRICTO:     {}",
        config.strict_mode.to_string().cyan()
    );
    println!(
        "  TIEMPO_ESPERA:     {}s",
        config.timeout.to_string().cyan()
    );

    match &config.pr_base_branch {
        Some(b) => println!("  RAMA_BASE_PR:      {}", b.cyan()),
        None => println!("  RAMA_BASE_PR:      {}", "auto-detectar".yellow()),
    }

    println!();

    output::log_section("Archivo de reglas:");

    if std::path::Path::new(&config.rules_file).exists() {
        println!("  {}", "Encontrado".green());
    } else {
        println!("  {} ({})", "No encontrado".red(), config.rules_file);
    }

    println!();

    Ok(())
}
