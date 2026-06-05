//! File system integration for resource management.
//!
//! Provides [`ResourceManager`] for scanning directories and building
//! a catalog of available assets with metadata.

use crate::types::{ResourceMeta, ResourceType};
use std::fs;
use std::path::{Path, PathBuf};

/// Resource manager that handles file system operations.
///
/// Recursively scans a root directory to build a list of available
/// resources with their metadata (type, size, path).
pub struct ResourceManager {
    root_path: PathBuf,
    resources: Vec<ResourceMeta>,
}

impl ResourceManager {
    /// Create a new resource manager rooted at the given path.
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            resources: Vec::new(),
        }
    }

    /// Rescan the root directory, rebuilding the resource list.
    pub fn refresh(&mut self) -> Result<(), String> {
        self.resources.clear();
        self.scan_directory(&self.root_path.clone())?;
        Ok(())
    }

    fn scan_directory(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory '{}': {}", path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|e| format!("Failed to get metadata: {}", e))?;

            let file_type = if metadata.is_dir() {
                ResourceType::Directory
            } else if let Some(ext) = path.extension() {
                ResourceType::from_extension(&ext.to_string_lossy())
            } else {
                ResourceType::Unknown
            };

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let size = if metadata.is_file() {
                metadata.len()
            } else {
                0
            };
            let last_modified = 0;

            self.resources.push(ResourceMeta {
                path: path.clone(),
                name,
                file_type,
                size,
                last_modified,
            });

            if metadata.is_dir() {
                self.scan_directory(&path)?;
            }
        }

        Ok(())
    }

    /// Get all discovered resources.
    pub fn get_resources(&self) -> &[ResourceMeta] {
        &self.resources
    }

    /// Get resources that are direct children of the given directory.
    pub fn get_resources_in_directory(&self, dir: &Path) -> Vec<&ResourceMeta> {
        self.resources
            .iter()
            .filter(|r| r.path.parent() == Some(dir))
            .collect()
    }

    /// Get the root path this manager is scanning.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new("./assets")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resource_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = ResourceManager::new(dir.path());
        assert_eq!(manager.root_path(), dir.path());
    }

    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let mut manager = ResourceManager::new(dir.path());
        manager.refresh().unwrap();
        assert_eq!(manager.get_resources().len(), 0);
    }

    #[test]
    fn test_scan_missing_directory() {
        let mut manager = ResourceManager::new("/nonexistent/path/that/does/not/exist");
        let result = manager.refresh();
        // Missing directory returns Ok (graceful skip) with empty resources
        assert!(result.is_ok());
        assert_eq!(manager.get_resources().len(), 0);
    }

    #[test]
    fn test_scan_nested_directories() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("file.txt"), "nested").unwrap();

        let mut manager = ResourceManager::new(dir.path());
        manager.refresh().unwrap();

        let resources = manager.get_resources();
        assert!(resources.len() >= 2); // sub dir + file.txt
        assert!(resources.iter().any(|r| r.name == "sub"));
        assert!(resources.iter().any(|r| r.name == "file.txt"));
    }

    #[test]
    fn test_get_resources_in_directory() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();

        let mut manager = ResourceManager::new(dir.path());
        manager.refresh().unwrap();

        let root_resources = manager.get_resources_in_directory(dir.path());
        assert_eq!(root_resources.len(), 3); // a.txt, b.txt, sub
    }
}
