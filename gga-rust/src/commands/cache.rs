//! Cache command implementation

use anyhow::Result;
use colored::Colorize;

use crate::cache::CacheManager;
use crate::config::ConfigLoader;
use crate::models::CacheAction;
use crate::utils::output;

/// Ejecutar el comando cache
pub fn execute_cache(action: &CacheAction) -> Result<()> {
    output::print_banner(crate::utils::defaults::VERSION);

    let cache = CacheManager::new()?;

    match action {
        CacheAction::Status => {
            let config = ConfigLoader::load().unwrap_or_default();
            let _stats = cache.get_stats(
                &[], // We don't need files for status
                &config.rules_file,
                ".gga",
            )?;

            output::log_section("Estado del caché:");
            println!();

            match cache.get_project_cache_dir() {
                Ok(Some(dir)) => {
                    println!(
                        "  Directorio de caché: {}",
                        dir.display().to_string().cyan()
                    );

                    if cache.is_cache_valid(&config.rules_file, ".gga")? {
                        println!("  Validez del caché: {}", "Válido".green());
                    } else {
                        println!(
                            "  Validez del caché: {}",
                            "Inválido (reglas o configuración cambiaron)".yellow()
                        );
                    }

                    // Contar archivos en caché
                    let files_dir = dir.join("files");
                    let cached_count = if files_dir.exists() {
                        std::fs::read_dir(&files_dir)
                            .map(|d| d.count())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    println!("  Archivos en caché: {}", cached_count.to_string().cyan());

                    // Mostrar tamaño del caché
                    let size = get_dir_size(&dir)?;
                    println!("  Tamaño del caché: {}", format_bytes(size).cyan());
                }
                _ => {
                    println!(
                        "  Caché del proyecto: {}",
                        "No inicializado (¿no estás en un repo git?)".yellow()
                    );
                }
            }
            println!();
        }
        CacheAction::Clear => {
            cache.invalidate()?;
            output::log_success("Caché del proyecto limpiado");
            println!();
        }
        CacheAction::ClearAll => {
            cache.clear_all()?;
            output::log_success("Todos los datos de caché limpiados");
            println!();
        }
    }

    Ok(())
}

/// Obtener tamaño del directorio
fn get_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                size += get_dir_size(&entry.path())?;
            } else {
                size += metadata.len();
            }
        }
    }

    Ok(size)
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
