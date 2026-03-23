//! Intelligent caching system for review results
//!
//! Cache invalidation strategy:
//! 1. File content changes (SHA256 hash)
//! 2. Rules file (AGENTS.md) changes
//! 3. Config file (.gga) changes

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::models::FileToReview;

/// Cache manager (SOLID: Single Responsibility)
pub struct CacheManager {
    cache_dir: PathBuf,
}

/// Cache status for a file
#[derive(Debug, Clone)]
pub enum CacheStatus {
    Hit,
    Miss,
    Invalid,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_files: usize,
    pub cached_files: usize,
    pub cache_dir: PathBuf,
    pub is_valid: bool,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        Ok(Self { cache_dir })
    }

    /// Get platform-specific cache directory (SOC: Platform concerns)
    fn get_cache_dir() -> Result<PathBuf> {
        // Windows: %LOCALAPPDATA%/gga/cache
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(local_appdata).join("gga").join("cache"));
        }

        // XDG: $XDG_CACHE_HOME/gga
        if let Some(xdg_cache) = std::env::var_os("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(xdg_cache).join("gga"));
        }

        // Fallback: ~/.cache/gga
        dirs::cache_dir()
            .map(|p| p.join("gga"))
            .context("Failed to determine cache directory")
    }

    /// Get project-specific cache directory based on git root
    pub fn get_project_cache_dir(&self) -> Result<Option<PathBuf>> {
        let project_id = self.get_project_id()?;

        match project_id {
            Some(id) => Ok(Some(self.cache_dir.join(id))),
            None => Ok(None),
        }
    }

    /// Get project identifier (hash of git root path) (KISS: Simple ID)
    fn get_project_id(&self) -> Result<Option<String>> {
        let git_root = self.find_git_root()?;

        match git_root {
            Some(root) => {
                let hash = Self::hash_string(root.to_string_lossy().as_ref());
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Find git repository root
    fn find_git_root(&self) -> Result<Option<PathBuf>> {
        // Try using git2 first
        match git2::Repository::open_from_env() {
            Ok(repo) => {
                let workdir = repo.workdir().map(|p| p.to_path_buf());
                Ok(workdir)
            }
            Err(_) => {
                // Fallback: walk up looking for .git
                let mut current = std::env::current_dir()?;
                loop {
                    if current.join(".git").exists() {
                        return Ok(Some(current));
                    }
                    if !current.pop() {
                        break;
                    }
                }
                Ok(None)
            }
        }
    }

    /// Compute SHA256 hash of file content
    pub fn hash_file(path: &Path) -> Result<String> {
        let content =
            std::fs::read(path).with_context(|| format!("Failed to read file: {:?}", path))?;
        Ok(Self::hash_bytes(&content))
    }

    /// Compute SHA256 hash of string
    pub fn hash_string(s: &str) -> String {
        Self::hash_bytes(s.as_bytes())
    }

    /// Compute SHA256 hash of bytes
    fn hash_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    /// Compute metadata hash (rules + config combined)
    pub fn compute_metadata_hash(&self, rules_file: &str, config_file: &str) -> Result<String> {
        let rules_hash = if Path::new(rules_file).exists() {
            Self::hash_file(Path::new(rules_file))?
        } else {
            String::new()
        };

        let config_hash = if Path::new(config_file).exists() {
            Self::hash_file(Path::new(config_file))?
        } else {
            String::new()
        };

        Ok(Self::hash_string(&format!(
            "{}:{}",
            rules_hash, config_hash
        )))
    }

    /// Check if cache is valid (metadata hasn't changed)
    pub fn is_cache_valid(&self, rules_file: &str, config_file: &str) -> Result<bool> {
        let project_cache_dir = match self.get_project_cache_dir()? {
            Some(dir) => dir,
            None => return Ok(false),
        };

        let metadata_file = project_cache_dir.join("metadata");
        if !metadata_file.exists() {
            return Ok(false);
        }

        let stored_hash =
            std::fs::read_to_string(&metadata_file).context("Failed to read cache metadata")?;

        let current_hash = self.compute_metadata_hash(rules_file, config_file)?;

        Ok(stored_hash.trim() == current_hash)
    }

    /// Check if a file is cached and still valid
    pub fn is_file_cached(&self, file: &FileToReview) -> Result<CacheStatus> {
        let project_cache_dir = match self.get_project_cache_dir()? {
            Some(dir) => dir,
            None => return Ok(CacheStatus::Invalid),
        };

        let cache_file = project_cache_dir.join("files").join(&file.hash);

        if !cache_file.exists() {
            return Ok(CacheStatus::Miss);
        }

        let status = std::fs::read_to_string(&cache_file)
            .unwrap_or_default()
            .trim()
            .to_string();

        if status == "PASSED" {
            Ok(CacheStatus::Hit)
        } else {
            Ok(CacheStatus::Miss)
        }
    }

    /// Guardar resultado de revisión de un archivo
    pub fn cache_result(&self, file: &FileToReview, passed: bool) -> Result<()> {
        let project_cache_dir = match self.get_project_cache_dir()? {
            Some(dir) => dir,
            None => return Ok(()),
        };

        let files_dir = project_cache_dir.join("files");
        std::fs::create_dir_all(&files_dir).context("Error al crear directorio de caché")?;

        let cache_file = files_dir.join(&file.hash);
        let status = if passed { "PASSED" } else { "FAILED" };

        std::fs::write(cache_file, status).context("Error al escribir archivo de caché")?;

        Ok(())
    }

    /// Inicializar caché con metadatos actuales
    pub fn init_cache(&self, rules_file: &str, config_file: &str) -> Result<PathBuf> {
        let project_cache_dir = self
            .get_project_cache_dir()?
            .context("No estás en un repositorio git")?;

        std::fs::create_dir_all(&project_cache_dir)
            .context("Error al crear directorio de caché")?;

        let metadata_hash = self.compute_metadata_hash(rules_file, config_file)?;
        std::fs::write(project_cache_dir.join("metadata"), metadata_hash)
            .context("Error al escribir metadatos de caché")?;

        info!("Caché inicializado en {:?}", project_cache_dir);
        Ok(project_cache_dir)
    }

    /// Invalidar (eliminar) caché del proyecto
    pub fn invalidate(&self) -> Result<()> {
        if let Some(project_cache_dir) = self.get_project_cache_dir()? {
            if project_cache_dir.exists() {
                std::fs::remove_dir_all(&project_cache_dir)
                    .context("Error al eliminar directorio de caché")?;
                info!("Caché invalidado");
            }
        }
        Ok(())
    }

    /// Clear all cache data
    pub fn clear_all(&self) -> Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir).context("Failed to clear cache")?;
            info!("All cache cleared");
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(
        &self,
        files: &[FileToReview],
        rules_file: &str,
        config_file: &str,
    ) -> Result<CacheStats> {
        let project_cache_dir = self
            .get_project_cache_dir()?
            .unwrap_or_else(|| self.cache_dir.join("unknown"));

        let mut cached_count = 0;
        for file in files {
            if let Ok(CacheStatus::Hit) = self.is_file_cached(file) {
                cached_count += 1;
            }
        }

        Ok(CacheStats {
            total_files: files.len(),
            cached_files: cached_count,
            cache_dir: project_cache_dir,
            is_valid: self.is_cache_valid(rules_file, config_file)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string() {
        let hash = CacheManager::hash_string("test");
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
    }
}
