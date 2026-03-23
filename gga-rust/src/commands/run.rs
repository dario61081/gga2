//! Run command implementation
//! 
//! This is the main code review command.

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::{debug, info};

use crate::cache::CacheManager;
use crate::config::ConfigLoader;
use crate::git::GitOperations;
use crate::models::{FileToReview, ReviewContext, ReviewStatus, RunArgs};
use crate::providers::{self, ProviderFactory};
use crate::utils::{output, patterns};

/// Execute the run command
pub async fn execute_run(args: &RunArgs) -> Result<()> {
    let version = crate::utils::defaults::VERSION;
    output::print_banner(version);
    
    // Load configuration
    let config = ConfigLoader::load()
        .context("Failed to load configuration")?;
    
    // Validate provider is configured
    let provider_str = config.provider
        .as_ref()
        .context("No provider configured. Run 'gga init' to create config.")?;
    
    info!("Using provider: {}", provider_str);
    output::log_info(format!("Provider: {}", provider_str));
    output::log_info(format!("Rules file: {}", config.rules_file));
    output::log_info(format!("File patterns: {}", config.file_patterns));
    
    if !config.exclude_patterns.is_empty() {
        output::log_info(format!("Exclude patterns: {}", config.exclude_patterns));
    }
    
    // Validate provider is available
    providers::validate_provider(provider_str).await
        .context("Provider validation failed")?;
    
    // Check rules file exists
    if !std::path::Path::new(&config.rules_file).exists() {
        output::log_error(format!("Rules file not found: {}", config.rules_file));
        std::process::exit(1);
    }
    
    // Determine mode and get files
    let (files, pr_range) = get_files_to_review(args, &config)?;
    
    if files.is_empty() {
        output::log_warning("No matching files to review");
        println!();
        return Ok(());
    }
    
    // Initialize cache
    let cache = CacheManager::new()?;
    let use_cache = !args.no_cache && !args.ci && !args.pr_mode;
    
    if use_cache {
        if !cache.is_cache_valid(&config.rules_file, ".gga")? {
            output::log_info("Cache invalidated (rules or config changed)");
            cache.invalidate()?;
        }
        cache.init_cache(&config.rules_file, ".gga")?;
    }
    
    // Filter cached files
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
        output::log_success(format!("{} file(s) passed from cache", cached_count));
    }
    
    if files_to_review.is_empty() {
        output::log_success("All files passed from cache!");
        println!();
        output::log_success("CODE REVIEW PASSED (cached)");
        println!();
        return Ok(());
    }
    
    // Display files to review
    output::log_section("Files to review:");
    for file in &files_to_review {
        println!("  - {}", file.path.display());
    }
    println!();
    
    // Read rules
    let rules = std::fs::read_to_string(&config.rules_file)
        .context("Failed to read rules file")?;
    
    // Build review context
    let context = ReviewContext {
        rules,
        files: files_to_review.clone(),
        commit_message: None,
    };
    
    // Build prompt
    let prompt = build_review_prompt(&context, &pr_range)?;
    
    // Execute review
    let provider = ProviderFactory::create(provider_str)?;
    let result = providers::execute_with_timeout(
        provider.as_ref(),
        &prompt,
        config.timeout,
    ).await?;
    
    // Display result
    println!("{}", result);
    println!();
    
    // Parse and handle result
    let status = parse_review_status(&result, config.strict_mode);
    
    match status {
        ReviewStatus::Passed => {
            // Cache passed files
            if use_cache {
                for file in &files_to_review {
                    cache.cache_result(file, true)?;
                }
            }
            output::log_success("CODE REVIEW PASSED");
            println!();
            Ok(())
        }
        ReviewStatus::Failed(msg) => {
            output::log_error("CODE REVIEW FAILED");
            println!();
            output::log_info("Fix the violations listed above before committing.");
            println!();
            std::process::exit(1);
        }
        ReviewStatus::Ambiguous => {
            output::log_warning("Could not determine review status");
            if config.strict_mode {
                output::log_error("STRICT MODE: Failing due to ambiguous response");
                println!();
                println!("Expected 'STATUS: PASSED' or 'STATUS: FAILED' in response.");
                println!("Set strict_mode = false in .gga to allow ambiguous responses.");
                println!();
                std::process::exit(1);
            } else {
                output::log_warning("Allowing commit (strict_mode = false)");
                println!();
                Ok(())
            }
        }
    }
}

/// Get files to review based on mode
fn get_files_to_review(
    args: &RunArgs,
    config: &crate::models::Config,
) -> Result<(Vec<std::path::PathBuf>, Option<crate::git::PRRange>)> {
    let files = if args.pr_mode {
        output::log_info("Mode: PR (full file review)");
        let range = GitOperations::get_pr_range(config.pr_base_branch.as_deref())?;
        output::log_info(format!("PR range: {}", range.range_string));
        let files = GitOperations::get_pr_files(
            &range,
            &config.file_patterns,
            &config.exclude_patterns,
        )?;
        (files, Some(range))
    } else if args.ci {
        output::log_info("Mode: CI (reviewing last commit)");
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

/// Build review prompt
fn build_review_prompt(
    context: &ReviewContext,
    pr_range: &Option<crate::git::PRRange>,
) -> Result<String> {
    let mut prompt = String::new();
    
    if let Some(range) = pr_range {
        prompt.push_str(&format!(
            "You are a code reviewer analyzing a pull request against the {} branch.\n\n",
            range.base
        ));
    } else {
        prompt.push_str("You are a code reviewer. Analyze the files below and validate they comply with the coding standards provided.\n\n");
    }
    
    prompt.push_str("=== CODING STANDARDS ===\n");
    prompt.push_str(&context.rules);
    prompt.push_str("\n=== END CODING STANDARDS ===\n\n");
    
    prompt.push_str("=== FILES TO REVIEW ===\n");
    
    for file in &context.files {
        prompt.push_str(&format!("\n--- FILE: {} ---\n", file.path.display()));
        prompt.push_str(&file.content);
    }
    
    prompt.push_str("\n=== END FILES ===\n\n");
    
    prompt.push_str("**IMPORTANT: Your response MUST include one of these lines near the beginning:**\n");
    prompt.push_str("STATUS: PASSED\n");
    prompt.push_str("STATUS: FAILED\n\n");
    prompt.push_str("**Begin with STATUS:**\n");
    
    Ok(prompt)
}

/// Parse review status from response
fn parse_review_status(result: &str, _strict: bool) -> ReviewStatus {
    // Check first 15 lines for STATUS
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