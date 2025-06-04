use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::archive::ArchiveEntry;

/// Utilities for path manipulation and directory analysis
pub struct PathUtils;

impl PathUtils {
    /// Find the common path prefix among a list of archive entries
    pub fn find_common_prefix(entries: &[ArchiveEntry]) -> Option<PathBuf> {
        if entries.is_empty() {
            return None;
        }

        // Get all file paths (excluding directories)
        let file_paths: Vec<&PathBuf> = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| &entry.path)
            .collect();

        if file_paths.is_empty() {
            return None;
        }

        // Start with the first path as the common prefix
        let mut common_prefix = file_paths[0].clone();

        // For each subsequent path, find the common components
        for path in file_paths.iter().skip(1) {
            common_prefix = Self::find_common_path_components(&common_prefix, path);
            
            // If no common prefix, break early
            if common_prefix.as_os_str().is_empty() {
                break;
            }
        }

        if common_prefix.as_os_str().is_empty() {
            None
        } else {
            Some(common_prefix)
        }
    }

    /// Find common path components between two paths
    fn find_common_path_components(path1: &Path, path2: &Path) -> PathBuf {
        let mut common = PathBuf::new();

        let components1: Vec<_> = path1.components().collect();
        let components2: Vec<_> = path2.components().collect();

        for (comp1, comp2) in components1.iter().zip(components2.iter()) {
            if comp1 == comp2 {
                common.push(comp1);
            } else {
                break;
            }
        }

        common
    }

    /// Analyze if directory structure has redundant nesting
    /// Returns the redundant path prefix if found
    pub fn analyze_redundant_folders(entries: &[ArchiveEntry]) -> Option<PathBuf> {
        if entries.is_empty() {
            return None;
        }

        // Get all file paths (non-directories)
        let file_paths: Vec<&PathBuf> = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| &entry.path)
            .collect();

        if file_paths.is_empty() {
            return None;
        }

        // Find the common prefix of all files
        let mut common_prefix = file_paths[0].clone();
        for path in file_paths.iter().skip(1) {
            common_prefix = Self::find_common_path_components(&common_prefix, path);
            if common_prefix.as_os_str().is_empty() {
                return None;
            }
        }

        // Check if this common prefix represents a redundant structure
        // A structure is redundant if the common prefix is a directory that contains
        // only one subdirectory at each level until we reach files
        if common_prefix.components().count() > 0 {
            // Analyze the directory structure to see if it's truly redundant
            let mut current_prefix = PathBuf::new();
            let mut found_redundant = false;

            for component in common_prefix.components() {
                current_prefix.push(component);
                
                // Check if at this level, there's only one subdirectory
                let children_at_level: HashSet<PathBuf> = file_paths
                    .iter()
                    .filter_map(|path| {
                        if let Ok(relative) = path.strip_prefix(&current_prefix) {
                            relative.components().next().map(|comp| {
                                current_prefix.join(comp.as_os_str())
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                // If there's only one unique child at this level, it might be redundant
                if children_at_level.len() == 1 {
                    found_redundant = true;
                } else {
                    break;
                }
            }

            if found_redundant && common_prefix != PathBuf::from("") {
                // Find the actual redundant prefix
                // Start from the beginning and find how many levels are truly redundant
                let mut redundant_prefix = PathBuf::new();
                
                for component in common_prefix.components() {
                    let test_prefix = redundant_prefix.join(component);
                    
                    // Check if all files have this as a prefix and if there's only one child directory
                    let remaining_paths: Vec<PathBuf> = file_paths
                        .iter()
                        .filter_map(|path| path.strip_prefix(&test_prefix).ok().map(|p| p.to_path_buf()))
                        .collect();
                    
                    if !remaining_paths.is_empty() {
                        let first_components: HashSet<_> = remaining_paths
                            .iter()
                            .filter_map(|path| path.components().next().map(|c| c.as_os_str()))
                            .collect();
                        
                        // If there's only one unique first component, this level is redundant
                        if first_components.len() == 1 {
                            redundant_prefix = test_prefix;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if !redundant_prefix.as_os_str().is_empty() {
                    return Some(redundant_prefix);
                }
            }
        }

        None
    }


    /// Remove redundant directory prefix from archive entries
    pub fn remove_redundant_prefix(
        entries: &mut Vec<ArchiveEntry>,
        redundant_prefix: &Path,
    ) -> Result<()> {
        debug!("Removing redundant prefix: {:?}", redundant_prefix);

        for entry in entries.iter_mut() {
            if let Ok(relative_path) = entry.path.strip_prefix(redundant_prefix) {
                // Save original path before modification
                if entry.original_path.is_none() {
                    entry.original_path = Some(entry.path.clone());
                }
                entry.path = relative_path.to_path_buf();
                debug!("Updated path: {:?}", entry.path);
            } else {
                warn!("Failed to strip prefix from path: {:?}", entry.path);
            }
        }

        // Remove any entries that would become empty paths
        entries.retain(|entry| !entry.path.as_os_str().is_empty());

        info!("Removed redundant prefix, {} entries remaining", entries.len());
        Ok(())
    }

    /// Generate a unique filename by appending a number if the file already exists
    pub fn generate_unique_filename(base_path: &Path) -> PathBuf {
        if !base_path.exists() {
            return base_path.to_path_buf();
        }

        let parent = base_path.parent().unwrap_or(Path::new("."));
        let file_stem = base_path.file_stem().unwrap_or_default();
        let extension = base_path.extension();

        for i in 1..=9999 {
            let new_name = if let Some(ext) = extension {
                format!("{}_{}.{}", file_stem.to_string_lossy(), i, ext.to_string_lossy())
            } else {
                format!("{}_{}", file_stem.to_string_lossy(), i)
            };

            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return new_path;
            }
        }

        // Fallback: use timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let new_name = if let Some(ext) = extension {
            format!("{}_{}.{}", file_stem.to_string_lossy(), timestamp, ext.to_string_lossy())
        } else {
            format!("{}_{}", file_stem.to_string_lossy(), timestamp)
        };

        parent.join(new_name)
    }

    /// Find all archive files in a directory (recursively if enabled)
    pub fn find_archive_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
        let mut archive_files = Vec::new();

        let walker = if recursive {
            WalkDir::new(dir).follow_links(false)
        } else {
            WalkDir::new(dir).max_depth(1).follow_links(false)
        };

        for entry in walker {
            let entry = entry.with_context(|| format!("Failed to read directory entry in: {:?}", dir))?;
            let path = entry.path();

            if path.is_file() && Self::is_supported_archive(path) {
                archive_files.push(path.to_path_buf());
            }
        }

        // Sort files for consistent processing order
        archive_files.sort();

        Ok(archive_files)
    }

    /// Check if a file is a supported archive format
    pub fn is_supported_archive(path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            let ext = extension.to_lowercase();
            match ext.as_str() {
                "zip" => true,
                #[cfg(feature = "rar")]
                "rar" => true,
                "7z" => true,
                #[cfg(feature = "lzh")]
                "lzh" | "lha" => true,
                "gz" | "bz2" => {
                    // Check for .tar.gz or .tar.bz2
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        stem.ends_with(".tar")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Clean up empty directories in a path
    pub fn remove_empty_directories(root_path: &Path) -> Result<usize> {
        let mut removed_count = 0;
        let mut dirs_to_check = Vec::new();

        // Collect all directories, deepest first
        for entry in WalkDir::new(root_path).follow_links(false) {
            let entry = entry.with_context(|| format!("Failed to walk directory: {:?}", root_path))?;
            if entry.file_type().is_dir() && entry.path() != root_path {
                dirs_to_check.push(entry.path().to_path_buf());
            }
        }

        // Sort by depth (deepest first)
        dirs_to_check.sort_by(|a, b| b.components().count().cmp(&a.components().count()));

        // Remove empty directories
        for dir in dirs_to_check {
            if Self::is_empty_directory(&dir)? {
                match std::fs::remove_dir(&dir) {
                    Ok(()) => {
                        debug!("Removed empty directory: {:?}", dir);
                        removed_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to remove empty directory {:?}: {}", dir, e);
                    }
                }
            }
        }

        if removed_count > 0 {
            info!("Removed {} empty directories", removed_count);
        }

        Ok(removed_count)
    }

    /// Check if a directory is empty
    fn is_empty_directory(dir: &Path) -> Result<bool> {
        let mut entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {:?}", dir))?;
        Ok(entries.next().is_none())
    }

    /// Normalize path separators for cross-platform compatibility
    pub fn normalize_path_separators(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let normalized = path_str.replace('\\', "/");
        PathBuf::from(normalized)
    }

    /// Convert Windows path to Unix-style path for archive storage
    pub fn to_archive_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// File filtering utilities
pub struct FileFilter;

impl FileFilter {
    /// Filter archive entries based on configuration rules
    pub fn filter_entries(
        entries: &mut Vec<ArchiveEntry>,
        include_pattern: Option<&regex::Regex>,
        exclude_pattern: Option<&regex::Regex>,
    ) -> usize {
        Self::filter_entries_with_filesystem(entries, include_pattern, exclude_pattern, None)
    }

    /// Filter archive entries and optionally delete files from filesystem
    pub fn filter_entries_with_filesystem(
        entries: &mut Vec<ArchiveEntry>,
        include_pattern: Option<&regex::Regex>,
        exclude_pattern: Option<&regex::Regex>,
        extract_path: Option<&Path>,
    ) -> usize {
        let initial_count = entries.len();
        let mut filtered_files = Vec::new();

        entries.retain(|entry| {
            // Always keep directory entries for now (we'll clean them up later)
            if entry.is_dir {
                return true;
            }

            let path_str = entry.path.to_string_lossy();

            // Check exclude pattern first (more specific)
            if let Some(exclude_regex) = exclude_pattern {
                if exclude_regex.is_match(&path_str) {
                    debug!("Excluding file: {}", path_str);
                    filtered_files.push(entry.clone());
                    return false;
                }
            }

            // Check include pattern (if specified, only include matching files)
            if let Some(include_regex) = include_pattern {
                if !include_regex.is_match(&path_str) {
                    debug!("Not including file: {}", path_str);
                    filtered_files.push(entry.clone());
                    return false;
                }
            }

            true
        });

        // Delete filtered files from filesystem if extract_path is provided
        if let Some(base_path) = extract_path {
            for entry in &filtered_files {
                // Use original path for filesystem lookup if available
                let filesystem_path = entry.original_path.as_ref().unwrap_or(&entry.path);
                let full_path = base_path.join(filesystem_path);
                
                if full_path.exists() {
                    if let Err(e) = std::fs::remove_file(&full_path) {
                        warn!("Failed to delete filtered file {:?}: {}", full_path, e);
                    } else {
                        debug!("Deleted filtered file from filesystem: {:?}", full_path);
                    }
                }
            }
        }

        let filtered_count = initial_count - entries.len();
        if filtered_count > 0 {
            info!("Filtered out {} files based on regex patterns", filtered_count);
        }

        filtered_count
    }

    /// Remove entries for empty directories
    pub fn remove_empty_directory_entries(entries: &mut Vec<ArchiveEntry>) -> usize {
        let initial_count = entries.len();

        // Build a set of all file paths for quick lookup
        let file_paths: HashSet<PathBuf> = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path.clone())
            .collect();

        // Remove directory entries that don't contain any files
        entries.retain(|entry| {
            if !entry.is_dir {
                return true; // Keep all files
            }

            // Check if this directory contains any files
            let dir_path = &entry.path;
            let has_files = file_paths.iter().any(|file_path| {
                file_path.starts_with(dir_path) && file_path != dir_path
            });

            if !has_files {
                debug!("Removing empty directory entry: {:?}", dir_path);
            }

            has_files
        });

        let removed_count = initial_count - entries.len();
        if removed_count > 0 {
            info!("Removed {} empty directory entries", removed_count);
        }

        removed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_entry(path: &str, is_dir: bool) -> ArchiveEntry {
        ArchiveEntry {
            path: PathBuf::from(path),
            original_path: None,
            size: if is_dir { 0 } else { 100 },
            compressed_size: if is_dir { 0 } else { 50 },
            modified_time: Some(SystemTime::now()),
            is_dir,
            is_encrypted: false,
        }
    }

    #[test]
    fn test_find_common_prefix() {
        let entries = vec![
            create_test_entry("folder1/folder2/file1.txt", false),
            create_test_entry("folder1/folder2/file2.txt", false),
            create_test_entry("folder1/folder2/subfolder/file3.txt", false),
        ];

        let common_prefix = PathUtils::find_common_prefix(&entries);
        assert_eq!(common_prefix, Some(PathBuf::from("folder1/folder2")));
    }

    #[test]
    fn test_find_common_prefix_no_common() {
        let entries = vec![
            create_test_entry("folder1/file1.txt", false),
            create_test_entry("folder2/file2.txt", false),
        ];

        let common_prefix = PathUtils::find_common_prefix(&entries);
        assert_eq!(common_prefix, None);
    }

    #[test]
    fn test_analyze_redundant_folders() {
        let entries = vec![
            create_test_entry("wrapper", true),
            create_test_entry("wrapper/content", true),
            create_test_entry("wrapper/content/file1.txt", false),
            create_test_entry("wrapper/content/file2.txt", false),
        ];

        let redundant = PathUtils::analyze_redundant_folders(&entries);
        assert_eq!(redundant, Some(PathBuf::from("wrapper")));
    }

    #[test]
    fn test_is_supported_archive() {
        assert!(PathUtils::is_supported_archive(Path::new("test.zip")));
        // assert!(PathUtils::is_supported_archive(Path::new("test.rar"))); // Temporarily disabled
        // assert!(PathUtils::is_supported_archive(Path::new("test.7z"))); // Temporarily disabled
        // assert!(PathUtils::is_supported_archive(Path::new("test.lzh"))); // Temporarily disabled
        assert!(PathUtils::is_supported_archive(Path::new("test.tar.gz")));
        assert!(PathUtils::is_supported_archive(Path::new("test.tar.bz2")));
        
        assert!(!PathUtils::is_supported_archive(Path::new("test.txt")));
        assert!(!PathUtils::is_supported_archive(Path::new("test.gz"))); // Not .tar.gz
        assert!(!PathUtils::is_supported_archive(Path::new("test")));
    }

    #[test]
    fn test_filter_entries() {
        let mut entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("file2.tmp", false),
            create_test_entry("document.doc", false),
            create_test_entry("image.jpg", false),
        ];

        let exclude_regex = regex::Regex::new(r".*\.tmp").unwrap();
        let include_regex = regex::Regex::new(r".*\.(txt|doc)").unwrap();

        let filtered_count = FileFilter::filter_entries(
            &mut entries,
            Some(&include_regex),
            Some(&exclude_regex),
        );

        assert_eq!(filtered_count, 2); // file2.tmp and image.jpg should be filtered out
        assert_eq!(entries.len(), 2);
        
        let remaining_files: Vec<String> = entries
            .iter()
            .map(|e| e.path.to_string_lossy().to_string())
            .collect();
        
        assert!(remaining_files.contains(&"file1.txt".to_string()));
        assert!(remaining_files.contains(&"document.doc".to_string()));
    }
}