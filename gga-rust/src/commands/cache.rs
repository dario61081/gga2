//! Cache command implementation

use anyhow::Result;
use colored::Colorize;

use crate::cache::CacheManager;
use crate::config::ConfigLoader;
use crate::models::CacheAction;
use crate::utils::output;

/// Execute the cache command
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

            output::log_section("Cache Status:");
            println!();

            match cache.get_project_cache_dir() {
                Ok(Some(dir)) => {
                    println!("  Cache directory: {}", dir.display().to_string().cyan());

                    if cache.is_cache_valid(&config.rules_file, ".gga")? {
                        println!("  Cache validity: {}", "Valid".green());
                    } else {
                        println!(
                            "  Cache validity: {}",
                            "Invalid (rules or config changed)".yellow()
                        );
                    }

                    // Count cached files
                    let files_dir = dir.join("files");
                    let cached_count = if files_dir.exists() {
                        std::fs::read_dir(&files_dir)
                            .map(|d| d.count())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    println!("  Cached files: {}", cached_count.to_string().cyan());

                    // Show cache size
                    let size = get_dir_size(&dir)?;
                    println!("  Cache size: {}", format_bytes(size).cyan());
                }
                _ => {
                    println!(
                        "  Project cache: {}",
                        "Not initialized (not in a git repo?)".yellow()
                    );
                }
            }
            println!();
        }
        CacheAction::Clear => {
            cache.invalidate()?;
            output::log_success("Cleared cache for current project");
            println!();
        }
        CacheAction::ClearAll => {
            cache.clear_all()?;
            output::log_success("Cleared all cache data");
            println!();
        }
    }

    Ok(())
}

/// Get directory size
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
