use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration settings for UnifyZip
#[derive(Debug, Clone)]
pub struct Config {
    /// Process priority class (0=Normal, 1=Idle, 2=High, 3=Realtime, 4=Below Normal, 5=Above Normal)
    pub priority_class: u8,
    
    /// Whether to recursively search subdirectories
    pub search_subfolders: bool,
    
    /// Whether to delete empty folders after processing
    pub deletes_empty_folder: bool,
    
    /// Whether to use recycle bin for deleted files
    pub uses_recycle_bin: bool,
    
    /// Redundant folder removal mode (0=disabled, 1=enabled, 2=use folder name as archive name)
    pub removes_redundant_folder: u8,
    
    /// Whether to show progress during operations
    pub shows_progress: bool,
    
    /// Compression level (-1=default, 0=store, 1=fast, 9=best)
    pub compress_level: i8,
    
    /// Whether to extract all files at once (batch mode)
    pub batch_decompress: bool,
    
    /// Regex pattern for files to include (if specified, only matching files are kept)
    pub append_file: Option<Regex>,
    
    /// Regex pattern for files to exclude
    pub exclude_file: Option<Regex>,
    
    /// File time mode (0=no change, 1=set to specific time)
    pub file_time_mode: u8,
    
    /// Folder time mode (0=no change, 1=set to specific time, 2=sync with newest file)
    pub folder_time_mode: u8,
    
    /// Specific file time to set (if file_time_mode == 1)
    pub file_time: Option<String>,
    
    /// Specific folder time to set (if folder_time_mode == 1)
    pub folder_time: Option<String>,
    
    /// File attributes to set (-1 = no change)
    pub file_attribute: i32,
    
    /// Folder attributes to set (-1 = no change)
    pub folder_attribute: i32,
    
    /// Regex pattern for remarkable files (files that require special attention)
    pub remarkable_file: Option<Regex>,
    
    /// Warning level for remarkable files (0=no warning, 1=warn if all files are remarkable, 2=warn if any file is remarkable)
    pub warning_level: u8,
    
    /// Whether to use recycle bin for individual files during batch decompress
    pub recycles_individual_files: bool,
    
    /// Whether to exit immediately without waiting for user input
    pub exits_immediately: bool,
    
    /// Whether to check if files are already processed before starting
    pub pre_check: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            priority_class: 0,
            search_subfolders: true,
            deletes_empty_folder: true,
            uses_recycle_bin: true,
            removes_redundant_folder: 1,
            shows_progress: true,
            compress_level: -1,
            batch_decompress: true,
            append_file: None,
            exclude_file: None,
            file_time_mode: 0,
            folder_time_mode: 2,
            file_time: None,
            folder_time: None,
            file_attribute: -1,
            folder_attribute: -1,
            remarkable_file: None,
            warning_level: 0,
            recycles_individual_files: false,
            exits_immediately: false,
            pre_check: true,
        }
    }
}

impl Config {
    /// Load configuration from INI file
    pub fn load(config_path: &Path) -> Result<Self> {
        let mut config = Self::default();

        if !config_path.exists() {
            log::info!("Configuration file not found, using defaults: {:?}", config_path);
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read configuration file: {:?}", config_path))?;

        let settings = parse_ini_contents(&contents);

        // Helper function to get config value
        let get_config = |key: &str| settings.get(key).cloned();

        if let Some(value) = get_config("PriorityClass") {
            config.priority_class = value.parse().unwrap_or(0);
        }

        if let Some(value) = get_config("SearchSubfolders") {
            config.search_subfolders = parse_bool(&value);
        }

        if let Some(value) = get_config("DeletesEmptyFolder") {
            config.deletes_empty_folder = parse_bool(&value);
        }

        if let Some(value) = get_config("UsesRecycleBin") {
            config.uses_recycle_bin = parse_bool(&value);
        }

        if let Some(value) = get_config("RemovesRedundantFolder") {
            config.removes_redundant_folder = value.parse().unwrap_or(1);
        }

        if let Some(value) = get_config("ShowsProgress") {
            config.shows_progress = parse_bool(&value);
        }

        if let Some(value) = get_config("CompressLevel") {
            config.compress_level = value.parse().unwrap_or(-1);
        }

        if let Some(value) = get_config("BatchDecompress") {
            config.batch_decompress = parse_bool(&value);
        }

        if let Some(value) = get_config("AppendFile") {
            if !value.is_empty() {
                match Regex::new(&value) {
                    Ok(regex) => config.append_file = Some(regex),
                    Err(e) => log::warn!("Invalid AppendFile regex '{}': {}", value, e),
                }
            }
        }

        if let Some(value) = get_config("ExcludeFile") {
            if !value.is_empty() {
                match Regex::new(&value) {
                    Ok(regex) => config.exclude_file = Some(regex),
                    Err(e) => log::warn!("Invalid ExcludeFile regex '{}': {}", value, e),
                }
            }
        }

        if let Some(value) = get_config("FileTimeMode") {
            config.file_time_mode = value.parse().unwrap_or(0);
        }

        if let Some(value) = get_config("FolderTimeMode") {
            config.folder_time_mode = value.parse().unwrap_or(2);
        }

        if let Some(value) = get_config("FileTime") {
            if !value.is_empty() {
                config.file_time = Some(value);
            }
        }

        if let Some(value) = get_config("FolderTime") {
            if !value.is_empty() {
                config.folder_time = Some(value);
            }
        }

        if let Some(value) = get_config("FileAttribute") {
            config.file_attribute = value.parse().unwrap_or(-1);
        }

        if let Some(value) = get_config("FolderAttribute") {
            config.folder_attribute = value.parse().unwrap_or(-1);
        }

        if let Some(value) = get_config("RemarkableFile") {
            if !value.is_empty() {
                match Regex::new(&value) {
                    Ok(regex) => config.remarkable_file = Some(regex),
                    Err(e) => log::warn!("Invalid RemarkableFile regex '{}': {}", value, e),
                }
            }
        }

        if let Some(value) = get_config("WarningLevel") {
            config.warning_level = value.parse().unwrap_or(0);
        }

        if let Some(value) = get_config("RecyclesIndividualFiles") {
            config.recycles_individual_files = parse_bool(&value);
        }

        if let Some(value) = get_config("ExitsImmediately") {
            config.exits_immediately = parse_bool(&value);
        }

        if let Some(value) = get_config("PreCheck") {
            config.pre_check = parse_bool(&value);
        }

        log::debug!("Loaded configuration: {:?}", config);
        Ok(config)
    }

    /// Create a default configuration file
    pub fn create_default(config_path: &Path) -> Result<()> {
        let default_content = r#"[DEFAULT]
; UnifyZip Configuration File
; This file controls the behavior of the UnifyZip archive processor

; Process priority class (0=Normal, 1=Idle, 2=High, 3=Realtime, 4=Below Normal, 5=Above Normal)
PriorityClass=0

; Whether to recursively search subdirectories (0=No, 1=Yes)
SearchSubfolders=1

; Whether to delete empty folders after processing (0=No, 1=Yes)
DeletesEmptyFolder=1

; Whether to use recycle bin for deleted files (0=No, 1=Yes)
UsesRecycleBin=1

; Redundant folder removal mode (0=disabled, 1=enabled, 2=use folder name as archive name)
RemovesRedundantFolder=1

; Whether to show progress during operations (0=No, 1=Yes)
ShowsProgress=1

; Compression level (-1=default, 0=store, 1=fast, 9=best)
CompressLevel=-1

; Whether to extract all files at once (0=No, 1=Yes)
BatchDecompress=1

; Regex pattern for files to include (empty = include all)
; Example: AppendFile=.*\.(txt|doc|pdf)
AppendFile=

; Regex pattern for files to exclude
; Example: ExcludeFile=.*\.(tmp|bak|~)
ExcludeFile=

; File time mode (0=no change, 1=set to specific time)
FileTimeMode=0

; Folder time mode (0=no change, 1=set to specific time, 2=sync with newest file)
FolderTimeMode=2

; Specific file time to set (if FileTimeMode=1)
; Format: YYYY/MM/DD HH:MM:SS
FileTime=

; Specific folder time to set (if FolderTimeMode=1)
; Format: YYYY/MM/DD HH:MM:SS
FolderTime=

; File attributes to set (-1 = no change)
; Values: 1=ReadOnly, 2=Hidden, 4=System, 32=Archive (can be combined)
FileAttribute=-1

; Folder attributes to set (-1 = no change)
FolderAttribute=-1

; Regex pattern for remarkable files (files that require special attention)
; Example: RemarkableFile=.*\.(exe|bat|cmd)
RemarkableFile=

; Warning level for remarkable files (0=no warning, 1=warn if all files are remarkable, 2=warn if any file is remarkable)
WarningLevel=0

; Whether to use recycle bin for individual files during batch decompress (0=No, 1=Yes)
RecyclesIndividualFiles=0

; Whether to exit immediately without waiting for user input (0=No, 1=Yes)
ExitsImmediately=0

; Whether to check if files are already processed before starting (0=No, 1=Yes)
PreCheck=1
"#;

        fs::write(config_path, default_content)
            .with_context(|| format!("Failed to create default configuration file: {:?}", config_path))?;

        log::info!("Created default configuration file: {:?}", config_path);
        Ok(())
    }

    /// Check if a file should be excluded based on ExcludeFile pattern
    pub fn should_exclude_file(&self, file_path: &str) -> bool {
        if let Some(ref regex) = self.exclude_file {
            regex.is_match(file_path)
        } else {
            false
        }
    }

    /// Check if a file should be included based on AppendFile pattern
    /// Returns true if no AppendFile pattern is set, or if the file matches the pattern
    pub fn should_include_file(&self, file_path: &str) -> bool {
        if let Some(ref regex) = self.append_file {
            regex.is_match(file_path)
        } else {
            true // Include all files if no pattern is specified
        }
    }

    /// Check if a file is remarkable based on RemarkableFile pattern
    pub fn is_remarkable_file(&self, file_path: &str) -> bool {
        if let Some(ref regex) = self.remarkable_file {
            regex.is_match(file_path)
        } else {
            false
        }
    }

    /// Get compression level as a value suitable for ZIP library
    pub fn get_zip_compression_level(&self) -> i32 {
        match self.compress_level {
            -1 => 6, // Default compression
            0 => 0,  // No compression (store)
            1..=9 => self.compress_level as i32,
            _ => 6,  // Fallback to default
        }
    }
}

/// Simple INI file parser
fn parse_ini_contents(contents: &str) -> HashMap<String, String> {
    let mut settings = HashMap::new();
    
    for line in contents.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        
        // Parse key=value pairs
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            settings.insert(key, value);
        }
    }
    
    settings
}

/// Parse a string value as boolean (supports "0"/"1" and "true"/"false")
fn parse_bool(value: &str) -> bool {
    match value.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.priority_class, 0);
        assert!(config.search_subfolders);
        assert!(config.deletes_empty_folder);
        assert!(config.uses_recycle_bin);
        assert_eq!(config.removes_redundant_folder, 1);
        assert!(config.shows_progress);
        assert_eq!(config.compress_level, -1);
        assert!(config.batch_decompress);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("1"));
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("on"));
        
        assert!(!parse_bool("0"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("FALSE"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("off"));
        assert!(!parse_bool("invalid"));
    }

    #[test]
    fn test_load_config_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "PriorityClass=2").unwrap();
        writeln!(temp_file, "SearchSubfolders=0").unwrap();
        writeln!(temp_file, "CompressLevel=9").unwrap();
        writeln!(temp_file, "ExcludeFile=.*\\.tmp").unwrap();
        
        let config = Config::load(temp_file.path()).unwrap();
        
        assert_eq!(config.priority_class, 2);
        assert!(!config.search_subfolders);
        assert_eq!(config.compress_level, 9);
        assert!(config.should_exclude_file("test.tmp"));
        assert!(!config.should_exclude_file("test.txt"));
    }

    #[test]
    fn test_should_exclude_file() {
        let mut config = Config::default();
        config.exclude_file = Some(Regex::new(r".*\.tmp").unwrap());
        
        assert!(config.should_exclude_file("test.tmp"));
        assert!(config.should_exclude_file("path/to/file.tmp"));
        assert!(!config.should_exclude_file("test.txt"));
        assert!(!config.should_exclude_file("tmp.txt"));
    }

    #[test]
    fn test_should_include_file() {
        let mut config = Config::default();
        
        // No pattern set - should include all files
        assert!(config.should_include_file("test.txt"));
        assert!(config.should_include_file("test.tmp"));
        
        // Pattern set - should only include matching files
        config.append_file = Some(Regex::new(r".*\.(txt|doc)").unwrap());
        assert!(config.should_include_file("test.txt"));
        assert!(config.should_include_file("document.doc"));
        assert!(!config.should_include_file("test.tmp"));
        assert!(!config.should_include_file("image.jpg"));
    }

    #[test]
    fn test_get_zip_compression_level() {
        let mut config = Config::default();
        
        config.compress_level = -1;
        assert_eq!(config.get_zip_compression_level(), 6);
        
        config.compress_level = 0;
        assert_eq!(config.get_zip_compression_level(), 0);
        
        config.compress_level = 5;
        assert_eq!(config.get_zip_compression_level(), 5);
        
        config.compress_level = 15; // Invalid value
        assert_eq!(config.get_zip_compression_level(), 6);
    }
}