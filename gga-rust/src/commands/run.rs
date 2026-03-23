//! Implementación del comando run
//! 
//! Este es el comando principal de revisión de código.

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::info;

use crate::cache::CacheManager;
use crate::config::ConfigLoader;
use crate::git::GitOperations;
use crate::models::{FileToReview, ReviewContext, ReviewStatus, RunArgs};
use crate::providers::{self, ProviderFactory};
use crate::utils::output;

/// Ejecutar el comando run
pub async fn execute_run(args: &RunArgs) -> Result<()> {
    let version = crate::utils::defaults::VERSION;
    output::print_banner(version);
    
    // Cargar configuración
    let config = ConfigLoader::load()
        .context("Error al cargar la configuración")?;
    
    // Validar que el proveedor esté configurado
    let provider_str = config.provider
        .as_ref()
        .context("No hay proveedor configurado. Ejecuta 'gga init' para crear la configuración.")?;
    
    info!("Usando proveedor: {}", provider_str);
    output::log_info(format!("Proveedor: {}", provider_str));
    output::log_info(format!("Archivo de reglas: {}", config.rules_file));
    output::log_info(format!("Patrones de archivos: {}", config.file_patterns));
    
    if !config.exclude_patterns.is_empty() {
        output::log_info(format!("Patrones de exclusión: {}", config.exclude_patterns));
    }
    
    // Validar que el proveedor esté disponible
    providers::validate_provider(provider_str).await
        .context("Validación del proveedor falló")?;
    
    // Verificar que el archivo de reglas existe
    if !std::path::Path::new(&config.rules_file).exists() {
        output::log_error(format!("Archivo de reglas no encontrado: {}", config.rules_file));
        std::process::exit(1);
    }
    
    // Determinar modo y obtener archivos
    let (files, pr_range) = get_files_to_review(args, &config)?;
    
    if files.is_empty() {
        output::log_warning("No hay archivos coincidentes para revisar");
        println!();
        return Ok(());
    }
    
    // Inicializar caché
    let cache = CacheManager::new()?;
    let use_cache = !args.no_cache && !args.ci && !args.pr_mode;
    
    if use_cache {
        if !cache.is_cache_valid(&config.rules_file, ".gga")? {
            output::log_info("Caché invalidado (reglas o configuración cambiaron)");
            cache.invalidate()?;
        }
        cache.init_cache(&config.rules_file, ".gga")?;
    }
    
    // Filtrar archivos en caché
    let mut files_to_review = Vec::new();
    let mut cached_count = 0;
    
    for file in &files {
        let file_to_review = FileToReview {
            path: file.clone(),
            content: if args.ci {
                crate::utils::files::read_file(file, false)?
            } else {
                crate::utils::files::read_file(file, true)?
            },
            hash: crate::utils::files::compute_hash(file)?,
        };
        
        if use_cache {
            match cache.is_file_cached(&file_to_review)? {
                crate::cache::CacheStatus::Hit => {
                    cached_count += 1;
                    continue;
                }
                _ => files_to_review.push(file_to_review),
            }
        } else {
            files_to_review.push(file_to_review);
        }
    }
    
    if cached_count > 0 {
        output::log_success(format!("{} archivo(s) pasaron del caché", cached_count));
    }
    
    if files_to_review.is_empty() {
        output::log_success("¡Todos los archivos pasaron del caché!");
        println!();
        output::log_success("REVISIÓN DE CÓDIGO APROBADA (caché)");
        println!();
        return Ok(());
    }
    
    // Mostrar archivos a revisar
    output::log_section("Archivos a revisar:");
    for file in &files_to_review {
        println!("  - {}", file.path.display());
    }
    println!();
    
    // Leer reglas
    let rules = std::fs::read_to_string(&config.rules_file)
        .context("Error al leer archivo de reglas")?;
    
    // Construir contexto de revisión
    let context = ReviewContext {
        rules,
        files: files_to_review.clone(),
        commit_message: None,
    };
    
    // Construir prompt
    let prompt = build_review_prompt(&context, &pr_range)?;
    
    // Ejecutar revisión
    let provider = ProviderFactory::create(provider_str)?;
    let result = providers::execute_with_timeout(
        provider.as_ref(),
        &prompt,
        config.timeout,
    ).await?;
    
    // Mostrar resultado
    println!("{}", result);
    println!();
    
    // Parsear y manejar resultado
    let status = parse_review_status(&result);
    
    match status {
        ReviewStatus::Passed => {
            // Guardar archivos en caché
            if use_cache {
                for file in &files_to_review {
                    cache.cache_result(file, true)?;
                }
            }
            output::log_success("REVISIÓN DE CÓDIGO APROBADA");
            println!();
            Ok(())
        }
        ReviewStatus::Failed(_msg) => {
            output::log_error("REVISIÓN DE CÓDIGO RECHAZADA");
            println!();
            output::log_info("Corrige las violaciones listadas arriba antes de hacer commit.");
            println!();
            std::process::exit(1);
        }
        ReviewStatus::Ambiguous => {
            output::log_warning("No se pudo determinar el estado de la revisión");
            if config.strict_mode {
                output::log_error("MODO ESTRICTO: Fallando debido a respuesta ambigua");
                println!();
                println!("Se esperaba 'STATUS: PASSED' o 'STATUS: FAILED' en la respuesta.");
                println!("Establece strict_mode = false en .gga para permitir respuestas ambiguas.");
                println!();
                std::process::exit(1);
            } else {
                output::log_warning("Permitiendo commit (strict_mode = false)");
                println!();
                Ok(())
            }
        }
    }
}

/// Obtener archivos a revisar según el modo
fn get_files_to_review(
    args: &RunArgs,
    config: &crate::models::Config,
) -> Result<(Vec<std::path::PathBuf>, Option<crate::git::PRRange>)> {
    let files = if args.pr_mode {
        output::log_info("Modo: PR (revisión completa de archivos)");
        let range = GitOperations::get_pr_range(config.pr_base_branch.as_deref())?;
        output::log_info(format!("Rango PR: {}", range.range_string));
        let files = GitOperations::get_pr_files(
            &range,
            &config.file_patterns,
            &config.exclude_patterns,
        )?;
        (files, Some(range))
    } else if args.ci {
        output::log_info("Modo: CI (revisando último commit)");
        let files = GitOperations::get_ci_files(
            &config.file_patterns,
            &config.exclude_patterns,
        )?;
        (files, None)
    } else {
        let files = GitOperations::get_staged_files(
            &config.file_patterns,
            &config.exclude_patterns,
        )?;
        (files, None)
    };
    
    Ok(files)
}

/// Construir prompt de revisión
fn build_review_prompt(
    context: &ReviewContext,
    pr_range: &Option<crate::git::PRRange>,
) -> Result<String> {
    let mut prompt = String::new();
    
    if let Some(range) = pr_range {
        prompt.push_str(&format!(
            "Eres un revisor de código analizando un pull request contra la rama {}.\n\n",
            range.base
        ));
    } else {
        prompt.push_str("Eres un revisor de código. Analiza los archivos a continuación y valida que cumplan con los estándares de codificación proporcionados.\n\n");
    }
    
    prompt.push_str("=== ESTÁNDARES DE CODIFICACIÓN ===\n");
    prompt.push_str(&context.rules);
    prompt.push_str("\n=== FIN DE ESTÁNDARES DE CODIFICACIÓN ===\n\n");
    
    prompt.push_str("=== ARCHIVOS A REVISAR ===\n");
    
    for file in &context.files {
        prompt.push_str(&format!("\n--- ARCHIVO: {} ---\n", file.path.display()));
        prompt.push_str(&file.content);
    }
    
    prompt.push_str("\n=== FIN DE ARCHIVOS ===\n\n");
    
    prompt.push_str("**IMPORTANTE: Tu respuesta DEBE incluir una de estas líneas cerca del inicio:**\n");
    prompt.push_str("STATUS: PASSED\n");
    prompt.push_str("STATUS: FAILED\n\n");
    prompt.push_str("**Comienza con STATUS:**\n");
    
    Ok(prompt)
}

/// Parsear estado de revisión desde la respuesta
fn parse_review_status(result: &str) -> ReviewStatus {
    // Verificar primeras 15 líneas por STATUS
    let header: String = result
        .lines()
        .take(15)
        .collect::<Vec<_>>()
        .join("\n");

    if header.contains("STATUS: PASSED") {
        ReviewStatus::Passed
    } else if header.contains("STATUS: FAILED") {
        ReviewStatus::Failed(result.to_string())
    } else {
        ReviewStatus::Ambiguous
    }
}