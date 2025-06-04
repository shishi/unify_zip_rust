use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::archive::{create_extractor, ArchiveEntry, ZipCreator};
use crate::config::Config;
use crate::utils::{FileFilter, PathUtils};

/// Main archive processor that handles the complete workflow
pub struct ArchiveProcessor {
    config: Config,
    dry_run: bool,
}

impl ArchiveProcessor {
    /// Create a new archive processor with the given configuration
    pub fn new(config: Config, dry_run: bool) -> Self {
        Self { config, dry_run }
    }

    /// Process a file or directory path
    pub fn process_path(&self, path: &Path) -> Result<usize> {
        if path.is_file() {
            if PathUtils::is_supported_archive(path) {
                match self.process_archive(path) {
                    Ok(()) => Ok(1),
                    Err(e) => {
                        error!("Failed to process archive {:?}: {}", path, e);
                        Ok(0)
                    }
                }
            } else {
                warn!("File is not a supported archive: {:?}", path);
                Ok(0)
            }
        } else if path.is_dir() {
            self.process_directory(path)
        } else {
            warn!("Path does not exist or is not accessible: {:?}", path);
            Ok(0)
        }
    }

    /// Process all archives in a directory
    fn process_directory(&self, dir_path: &Path) -> Result<usize> {
        info!("Processing directory: {:?}", dir_path);

        let archive_files = PathUtils::find_archive_files(dir_path, self.config.search_subfolders)
            .with_context(|| format!("Failed to find archive files in: {:?}", dir_path))?;

        if archive_files.is_empty() {
            info!("No archive files found in directory: {:?}", dir_path);
            return Ok(0);
        }

        info!("Found {} archive files to process", archive_files.len());

        let mut processed_count = 0;
        for archive_path in archive_files {
            match self.process_archive(&archive_path) {
                Ok(()) => {
                    processed_count += 1;
                    info!("Successfully processed: {:?}", archive_path);
                }
                Err(e) => {
                    error!("Failed to process archive {:?}: {}", archive_path, e);
                }
            }
        }

        Ok(processed_count)
    }

    /// Process a single archive file
    fn process_archive(&self, archive_path: &Path) -> Result<()> {
        info!("Processing archive: {:?}", archive_path);

        if self.dry_run {
            info!("DRY RUN: Would process archive: {:?}", archive_path);
            return Ok(());
        }

        // Pre-check if archive needs processing
        if self.config.pre_check && self.is_already_processed(archive_path)? {
            info!("Archive already processed, skipping: {:?}", archive_path);
            return Ok(());
        }

        // Step 1: Extract archive to temporary directory
        let temp_dir = TempDir::new()
            .with_context(|| "Failed to create temporary directory")?;
        let extract_path = temp_dir.path();

        let mut entries = self.extract_archive(archive_path, extract_path)?;

        if entries.is_empty() {
            warn!("Archive contains no files: {:?}", archive_path);
            return Ok(());
        }

        info!("Extracted {} entries from archive", entries.len());

        // Step 2: Filter files based on configuration
        self.filter_entries(&mut entries, extract_path)?;

        if entries.is_empty() {
            warn!("No files remaining after filtering: {:?}", archive_path);
            return Ok(());
        }

        // Step 3: Remove redundant directories
        self.remove_redundant_directories(&mut entries)?;

        // Step 4: Check for remarkable files and warn if necessary
        self.check_remarkable_files(&entries)?;

        // Step 5: Create new ZIP archive
        let output_path = self.create_output_archive(&entries, extract_path, archive_path)?;

        // Step 6: Move original archive to trash/recycle bin
        self.dispose_original_archive(archive_path)?;

        // Step 7: Move new archive to final location
        self.finalize_archive(&output_path, archive_path)?;

        info!("Successfully processed archive: {:?}", archive_path);
        Ok(())
    }

    /// Extract archive to temporary directory
    fn extract_archive(&self, archive_path: &Path, extract_path: &Path) -> Result<Vec<ArchiveEntry>> {
        debug!("Extracting archive to: {:?}", extract_path);

        let extractor = create_extractor(archive_path.to_path_buf())
            .with_context(|| format!("Failed to create extractor for: {:?}", archive_path))?;

        let entries = if self.config.batch_decompress {
            // Extract all files at once
            extractor.extract_to(extract_path)?
        } else {
            // List entries first, then extract individually
            let entry_list = extractor.list_entries()?;
            extractor.extract_to(extract_path)?;
            entry_list
        };

        debug!("Extraction completed, {} entries extracted", entries.len());
        Ok(entries)
    }

    /// Filter entries based on configuration rules
    fn filter_entries(&self, entries: &mut Vec<ArchiveEntry>, extract_path: &Path) -> Result<()> {
        debug!("Filtering entries based on configuration");

        // Apply file filtering based on regex patterns, deleting files from filesystem
        let filtered_count = FileFilter::filter_entries_with_filesystem(
            entries,
            self.config.append_file.as_ref(),
            self.config.exclude_file.as_ref(),
            Some(extract_path), // Pass extract path for filesystem deletion
        );

        if filtered_count > 0 {
            info!("Filtered out {} files based on patterns", filtered_count);
        }

        // Remove empty directory entries if configured
        if self.config.deletes_empty_folder {
            let removed_count = FileFilter::remove_empty_directory_entries(entries);
            if removed_count > 0 {
                info!("Removed {} empty directory entries", removed_count);
            }
        }

        Ok(())
    }

    /// Remove redundant directory structures
    fn remove_redundant_directories(&self, entries: &mut Vec<ArchiveEntry>) -> Result<()> {
        if self.config.removes_redundant_folder == 0 {
            return Ok(()); // Redundant folder removal disabled
        }

        debug!("Analyzing redundant directories");

        if let Some(redundant_prefix) = PathUtils::analyze_redundant_folders(entries) {
            info!("Found redundant directory structure: {:?}", redundant_prefix);
            PathUtils::remove_redundant_prefix(entries, &redundant_prefix)?;
        } else {
            debug!("No redundant directory structure found");
        }

        Ok(())
    }

    /// Check for remarkable files and issue warnings if necessary
    fn check_remarkable_files(&self, entries: &[ArchiveEntry]) -> Result<()> {
        if self.config.warning_level == 0 || self.config.remarkable_file.is_none() {
            return Ok(());
        }

        let remarkable_files: Vec<&ArchiveEntry> = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .filter(|entry| self.config.is_remarkable_file(&entry.path.to_string_lossy()))
            .collect();

        let total_files = entries.iter().filter(|entry| !entry.is_dir).count();

        let should_warn = match self.config.warning_level {
            1 => remarkable_files.len() == total_files && total_files > 0, // All files are remarkable
            2 => !remarkable_files.is_empty(), // Any file is remarkable
            _ => false,
        };

        if should_warn {
            warn!("Archive contains remarkable files:");
            for file in &remarkable_files {
                warn!("  - {:?}", file.path);
            }

            if !self.config.exits_immediately {
                // In a real implementation, you might want to pause for user input here
                info!("Press Enter to continue...");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
            }
        }

        Ok(())
    }

    /// Create new ZIP archive with processed contents
    fn create_output_archive(
        &self,
        entries: &[ArchiveEntry],
        extract_path: &Path,
        original_archive_path: &Path,
    ) -> Result<PathBuf> {
        debug!("Creating new ZIP archive");

        let mut zip_creator = ZipCreator::new()
            .with_context(|| "Failed to create ZIP creator")?;

        // Add all entries to the new archive
        for entry in entries {
            // Use original path for filesystem lookup, current path for archive structure
            let filesystem_path = entry.original_path.as_ref().unwrap_or(&entry.path);
            let source_path = extract_path.join(filesystem_path);
            
            if source_path.exists() {
                zip_creator.add_path(&source_path, &entry.path)
                    .with_context(|| format!("Failed to add path to archive: {:?}", entry.path))?;
            } else {
                warn!("Source file not found during recompression: {:?}", source_path);
            }
        }

        // Determine output filename
        let output_filename = self.determine_output_filename(original_archive_path, entries)?;
        let temp_output_path = extract_path.join("output.zip");

        // Create the ZIP file
        zip_creator.create(&temp_output_path, self.config.get_zip_compression_level())
            .with_context(|| "Failed to create ZIP archive")?;

        // Move to final temporary location with correct name
        let final_temp_path = extract_path.join(&output_filename);
        std::fs::rename(&temp_output_path, &final_temp_path)
            .with_context(|| "Failed to rename output archive")?;

        info!("Created new archive: {:?}", output_filename);
        Ok(final_temp_path)
    }

    /// Determine the output filename for the new archive
    fn determine_output_filename(
        &self,
        original_path: &Path,
        entries: &[ArchiveEntry],
    ) -> Result<String> {
        let base_name = original_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive");

        let output_name = if self.config.removes_redundant_folder == 2 {
            // Use the name of the top-level folder if it exists
            if let Some(top_folder) = self.find_top_level_folder(entries) {
                format!("{}.zip", top_folder)
            } else {
                format!("{}.zip", base_name)
            }
        } else {
            format!("{}.zip", base_name)
        };

        Ok(output_name)
    }

    /// Find the top-level folder name from entries
    fn find_top_level_folder(&self, entries: &[ArchiveEntry]) -> Option<String> {
        for entry in entries {
            if entry.is_dir && entry.path.components().count() == 1 {
                return entry.path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());
            }
        }
        None
    }

    /// Move original archive to trash/recycle bin
    fn dispose_original_archive(&self, archive_path: &Path) -> Result<()> {
        if self.config.uses_recycle_bin {
            debug!("Moving original archive to recycle bin: {:?}", archive_path);
            trash::delete(archive_path)
                .with_context(|| format!("Failed to move to recycle bin: {:?}", archive_path))?;
            info!("Moved original archive to recycle bin");
        } else {
            debug!("Deleting original archive: {:?}", archive_path);
            std::fs::remove_file(archive_path)
                .with_context(|| format!("Failed to delete file: {:?}", archive_path))?;
            info!("Deleted original archive");
        }
        Ok(())
    }

    /// Move the new archive to its final location
    fn finalize_archive(&self, temp_output_path: &Path, original_archive_path: &Path) -> Result<()> {
        let final_path = original_archive_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(temp_output_path.file_name().unwrap());

        // Generate unique filename if file already exists
        let unique_path = PathUtils::generate_unique_filename(&final_path);

        debug!("Moving new archive to final location: {:?}", unique_path);
        std::fs::copy(temp_output_path, &unique_path)
            .with_context(|| format!("Failed to copy archive to final location: {:?}", unique_path))?;

        info!("New archive created at: {:?}", unique_path);
        Ok(())
    }

    /// Check if an archive has already been processed (simplified heuristic)
    fn is_already_processed(&self, archive_path: &Path) -> Result<bool> {
        // Simple heuristic: if it's already a ZIP file and relatively small compression ratio
        if let Some(extension) = archive_path.extension().and_then(|s| s.to_str()) {
            if extension.to_lowercase() == "zip" {
                // For now, assume ZIP files might already be processed
                // In a full implementation, you'd analyze the archive contents more thoroughly
                debug!("Archive is already ZIP format, might be processed: {:?}", archive_path);
                return Ok(false); // Still process for now
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_determine_output_filename() {
        let config = Config::default();
        let processor = ArchiveProcessor::new(config, false);
        
        let original_path = Path::new("/test/archive.rar");
        let entries = vec![];
        
        let filename = processor.determine_output_filename(original_path, &entries).unwrap();
        assert_eq!(filename, "archive.zip");
    }

    #[test]
    fn test_determine_output_filename_with_folder() {
        let mut config = Config::default();
        config.removes_redundant_folder = 2;
        let processor = ArchiveProcessor::new(config, false);
        
        let original_path = Path::new("/test/archive.rar");
        let entries = vec![
            ArchiveEntry {
                path: PathBuf::from("content"),
                original_path: None,
                size: 0,
                compressed_size: 0,
                modified_time: None,
                is_dir: true,
                is_encrypted: false,
            }
        ];
        
        let filename = processor.determine_output_filename(original_path, &entries).unwrap();
        assert_eq!(filename, "content.zip");
    }

    #[test]
    fn test_process_path_dry_run() {
        let config = Config::default();
        let processor = ArchiveProcessor::new(config, true);
        
        // Create a temporary ZIP file
        let temp_dir = tempdir().unwrap();
        let zip_path = temp_dir.path().join("test.zip");
        File::create(&zip_path).unwrap();
        
        // Dry run should succeed without error
        let result = processor.process_path(&zip_path).unwrap();
        assert_eq!(result, 1);
    }
}