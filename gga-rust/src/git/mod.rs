//! Git operations module
//!
//! Handles all git-related operations including:
//! - Getting staged files
//! - Getting files changed in PR range
//! - Installing/uninstalling git hooks
//! - PR range detection

use anyhow::{Context, Result};
use colored::Colorize;
use glob::Pattern;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::models::FileToReview;

/// Git operations handler (SOLID: Single Responsibility)
pub struct GitOperations;

/// PR range information
#[derive(Debug, Clone)]
pub struct PRRange {
    pub base: String,
    pub head: String,
    pub range_string: String,
}

impl GitOperations {
    /// Get staged files matching patterns
    pub fn get_staged_files(
        include_patterns: &str,
        exclude_patterns: &str,
    ) -> Result<Vec<PathBuf>> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let index = repo.index().context("Failed to get git index")?;

        let include = Self::parse_patterns(include_patterns);
        let exclude = Self::parse_patterns(exclude_patterns);

        let mut files = Vec::new();

        for entry in index.iter() {
            let path = String::from_utf8_lossy(&entry.path);
            let path_buf = PathBuf::from(path.as_ref());

            if Self::matches_patterns(&path_buf, &include)
                && !Self::matches_patterns(&path_buf, &exclude)
            {
                files.push(path_buf);
            }
        }

        debug!("Found {} staged files", files.len());
        Ok(files)
    }

    /// Get files changed in last commit (CI mode)
    pub fn get_ci_files(include_patterns: &str, exclude_patterns: &str) -> Result<Vec<PathBuf>> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let head = repo.head().context("Failed to get HEAD")?;

        let head_commit = head.peel_to_commit().context("Failed to get HEAD commit")?;

        let parent = head_commit
            .parent(0)
            .context("Failed to get parent commit")?;

        let head_tree = head_commit.tree().context("Failed to get HEAD tree")?;

        let parent_tree = parent.tree().context("Failed to get parent tree")?;

        let diff = repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), None)
            .context("Failed to create diff")?;

        let include = Self::parse_patterns(include_patterns);
        let exclude = Self::parse_patterns(exclude_patterns);

        let mut files = Vec::new();

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    let path_buf = path.to_path_buf();
                    if Self::matches_patterns(&path_buf, &include)
                        && !Self::matches_patterns(&path_buf, &exclude)
                        && path_buf.exists()
                    {
                        files.push(path_buf);
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;

        debug!("Found {} files in last commit", files.len());
        Ok(files)
    }

    /// Get files changed in PR range
    pub fn get_pr_files(
        range: &PRRange,
        include_patterns: &str,
        exclude_patterns: &str,
    ) -> Result<Vec<PathBuf>> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let base = repo
            .revparse_single(&range.base)
            .context("Failed to resolve base branch")?
            .peel_to_commit()
            .context("Failed to get base commit")?;

        let head = repo
            .revparse_single(&range.head)
            .context("Failed to resolve HEAD")?
            .peel_to_commit()
            .context("Failed to get HEAD commit")?;

        let base_tree = base.tree().context("Failed to get base tree")?;

        let head_tree = head.tree().context("Failed to get HEAD tree")?;

        let diff = repo
            .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
            .context("Failed to create diff")?;

        let include = Self::parse_patterns(include_patterns);
        let exclude = Self::parse_patterns(exclude_patterns);

        let mut files = Vec::new();

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    let path_buf = path.to_path_buf();
                    if Self::matches_patterns(&path_buf, &include)
                        && !Self::matches_patterns(&path_buf, &exclude)
                        && path_buf.exists()
                    {
                        files.push(path_buf);
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;

        debug!(
            "Found {} files in PR range {}",
            files.len(),
            range.range_string
        );
        Ok(files)
    }

    /// Get PR range by auto-detecting base branch
    pub fn get_pr_range(base_branch: Option<&str>) -> Result<PRRange> {
        let base = match base_branch {
            Some(branch) => branch.to_string(),
            None => Self::detect_base_branch()?,
        };

        let range_string = format!("{}...HEAD", base);

        Ok(PRRange {
            base,
            head: "HEAD".to_string(),
            range_string,
        })
    }

    /// Auto-detect base branch (main > master > develop)
    pub fn detect_base_branch() -> Result<String> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let branches = repo
            .branches(Some(git2::BranchType::Local))
            .context("Failed to list branches")?;

        let candidates = ["main", "master", "develop"];

        for branch in branches {
            let (branch, _) = branch.context("Failed to get branch")?;
            if let Ok(Some(name)) = branch.name() {
                for candidate in &candidates {
                    if name == *candidate {
                        return Ok(name.to_string());
                    }
                }
            }
        }

        warn!("Could not detect base branch (no main/master/develop found)");
        anyhow::bail!("Could not detect base branch. Set PR_BASE_BRANCH in .gga config.");
    }

    /// Get the diff for a PR range
    pub fn get_pr_diff(range: &PRRange) -> Result<String> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let base = repo
            .revparse_single(&range.base)
            .context("Failed to resolve base")?
            .peel_to_commit()
            .context("Failed to get base commit")?;

        let head = repo
            .revparse_single(&range.head)
            .context("Failed to resolve HEAD")?
            .peel_to_commit()
            .context("Failed to get HEAD commit")?;

        let base_tree = base.tree().context("Failed to get base tree")?;

        let head_tree = head.tree().context("Failed to get HEAD tree")?;

        let diff = repo
            .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
            .context("Failed to create diff")?;

        let mut output = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            output.extend_from_slice(line.content());
            true
        })?;

        Ok(String::from_utf8_lossy(&output).to_string())
    }

    /// Read file content from git staging area
    pub fn read_staged_file(path: &Path) -> Result<String> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let index = repo.index().context("Failed to get git index")?;

        let entry = index
            .get_path(path, 0)
            .context(format!("File not staged: {:?}", path))?;

        let blob = repo.find_blob(entry.id).context("Failed to read blob")?;

        let content = std::str::from_utf8(blob.content()).context("File is not valid UTF-8")?;

        Ok(content.to_string())
    }

    /// Read file content from filesystem
    pub fn read_file(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))
    }

    /// Parse comma-separated glob patterns
    fn parse_patterns(patterns: &str) -> Vec<Pattern> {
        patterns
            .split(',')
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    None
                } else {
                    Pattern::new(p).ok()
                }
            })
            .collect()
    }

    /// Check if path matches any pattern
    fn matches_patterns(path: &Path, patterns: &[Pattern]) -> bool {
        if patterns.is_empty() {
            return true;
        }

        let path_str = path.to_string_lossy();
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        patterns
            .iter()
            .any(|pattern| pattern.matches(&path_str) || pattern.matches(&filename))
    }

    /// Install git hook
    pub fn install_hook(hook_type: &str, use_commit_msg: bool) -> Result<()> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let git_path = repo.path().to_path_buf();

        let hooks_dir = git_path.join("hooks");
        std::fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;

        let hook_path = hooks_dir.join(hook_type);

        let gga_run_cmd = if use_commit_msg {
            r#"gga run "$1" || exit 1"#
        } else {
            r#"gga run || exit 1"#
        };

        let hook_content = format!(
            r#"#!/usr/bin/env bash

# ======== GGA START ========
# Gentleman Guardian Angel - Code Review
{}
# ======== GGA END ========
"#,
            gga_run_cmd
        );

        // Check if hook already exists with GGA markers
        if hook_path.exists() {
            let content = std::fs::read_to_string(&hook_path).unwrap_or_default();

            if content.contains("# ======== GGA START ========") {
                info!("GGA hook already installed in {}", hook_type);
                return Ok(());
            }

            // Existing hook - append GGA section
            warn!("Existing {} hook found, appending GGA section", hook_type);
            let new_content = format!("{}\n{}", content, hook_content);
            std::fs::write(&hook_path, new_content).context("Failed to update hook")?;
        } else {
            // Create new hook
            std::fs::write(&hook_path, hook_content).context("Failed to create hook")?;
        }

        // Make executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
                .context("Failed to set hook permissions")?;
        }

        info!("Installed {} hook: {:?}", hook_type, hook_path);
        Ok(())
    }

    /// Uninstall git hook
    pub fn uninstall_hook(hook_type: &str) -> Result<()> {
        let repo = git2::Repository::open_from_env().context("Not a git repository")?;

        let git_path = repo.path().to_path_buf();

        let hook_path = git_path.join("hooks").join(hook_type);

        if !hook_path.exists() {
            warn!("Hook not found: {:?}", hook_path);
            return Ok(());
        }

        let content = std::fs::read_to_string(&hook_path).context("Failed to read hook")?;

        // Check for GGA markers
        if !content.contains("# ======== GGA START ========") {
            info!("GGA not found in {} hook", hook_type);
            return Ok(());
        }

        // Remove GGA section
        let lines: Vec<&str> = content.lines().collect();
        let mut in_gga_section = false;
        let mut new_lines = Vec::new();

        for line in lines {
            if line.contains("# ======== GGA START ========") {
                in_gga_section = true;
                continue;
            }
            if line.contains("# ======== GGA END ========") {
                in_gga_section = false;
                continue;
            }
            if !in_gga_section {
                new_lines.push(line);
            }
        }

        let new_content = new_lines.join("\n");
        let trimmed = new_content.trim();

        if trimmed.is_empty() || trimmed == "#!/usr/bin/env bash" {
            // Hook was only GGA, remove it
            std::fs::remove_file(&hook_path).context("Failed to remove hook")?;
            info!("Removed {} hook (was GGA-only)", hook_type);
        } else {
            // Keep remaining content
            std::fs::write(&hook_path, trimmed).context("Failed to update hook")?;
            info!("Removed GGA from {} hook", hook_type);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_patterns() {
        let patterns = GitOperations::parse_patterns("*.rs,*.ts,*.tsx");
        assert_eq!(patterns.len(), 3);
    }
}
