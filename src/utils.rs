use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::archive::ArchiveEntry;

/// Utilities for path manipulation and directory analysis
pub struct PathUtils;

impl PathUtils {
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

    /// Find the redundant folder prefix shared by every entry.
    ///
    /// Mirrors the original UnifyZip behaviour: the redundant path is the
    /// deepest directory that contains every file and every empty directory.
    /// Directories that contain other entries carry no information of their
    /// own. Returns `None` when the archive has no files or when the entries
    /// do not all live under one top-level directory.
    pub fn analyze_redundant_folders(entries: &[ArchiveEntry]) -> Option<PathBuf> {
        if !entries.iter().any(|entry| !entry.is_dir) {
            return None;
        }

        let ancestors: HashSet<&Path> = entries
            .iter()
            .flat_map(|entry| entry.path.ancestors().skip(1))
            .collect();

        let mut common: Option<PathBuf> = None;
        for entry in entries {
            if entry.is_dir && ancestors.contains(entry.path.as_path()) {
                continue;
            }
            let dir = entry.path.parent().map(Path::to_path_buf).unwrap_or_default();
            let prefix = match common {
                None => dir,
                Some(prefix) => Self::find_common_path_components(&prefix, &dir),
            };
            if prefix.as_os_str().is_empty() {
                return None;
            }
            common = Some(prefix);
        }
        common
    }

    /// Remove redundant directory prefix from archive entries
    pub fn remove_redundant_prefix(
        entries: &mut Vec<ArchiveEntry>,
        redundant_prefix: &Path,
    ) -> Result<()> {
        debug!("Removing redundant prefix: {:?}", redundant_prefix);

        entries.retain_mut(|entry| match entry.path.strip_prefix(redundant_prefix) {
            // The redundant folder itself disappears
            Ok(relative_path) if relative_path.as_os_str().is_empty() => false,
            Ok(relative_path) => {
                // Save original path before modification
                if entry.original_path.is_none() {
                    entry.original_path = Some(entry.path.clone());
                }
                entry.path = relative_path.to_path_buf();
                debug!("Updated path: {:?}", entry.path);
                true
            }
            // Directories above the redundant folder disappear with it
            Err(_) if redundant_prefix.starts_with(&entry.path) => false,
            Err(_) => {
                warn!("Failed to strip prefix from path: {:?}", entry.path);
                true
            }
        });

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
    fn test_analyze_redundant_folders() {
        let entries = vec![
            create_test_entry("wrapper", true),
            create_test_entry("wrapper/content", true),
            create_test_entry("wrapper/content/file1.txt", false),
            create_test_entry("wrapper/content/file2.txt", false),
        ];

        let redundant = PathUtils::analyze_redundant_folders(&entries);
        assert_eq!(redundant, Some(PathBuf::from("wrapper/content")));
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