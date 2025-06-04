use std::fs;
use std::path::Path;
use tempfile::tempdir;
use unify_zip::config::Config;

#[test]
fn test_config_parsing() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("test_config.ini");
    
    // Create test config file
    let test_config = r#"[DEFAULT]
PriorityClass=2
SearchSubfolders=0
DeletesEmptyFolder=1
UsesRecycleBin=1
RemovesRedundantFolder=2
ShowsProgress=0
CompressLevel=9
BatchDecompress=0
AppendFile=.*\.(txt|doc)
ExcludeFile=.*\.(tmp|bak)
FileTimeMode=1
FolderTimeMode=0
FileAttribute=32
FolderAttribute=16
RemarkableFile=.*\.exe
WarningLevel=2
RecyclesIndividualFiles=1
ExitsImmediately=1
PreCheck=0
"#;
    
    fs::write(&config_path, test_config).unwrap();
    
    // Load and test configuration
    let config = Config::load(&config_path).unwrap();
    
    assert_eq!(config.priority_class, 2);
    assert_eq!(config.search_subfolders, false);
    assert_eq!(config.deletes_empty_folder, true);
    assert_eq!(config.uses_recycle_bin, true);
    assert_eq!(config.removes_redundant_folder, 2);
    assert_eq!(config.shows_progress, false);
    assert_eq!(config.compress_level, 9);
    assert_eq!(config.batch_decompress, false);
    assert_eq!(config.file_time_mode, 1);
    assert_eq!(config.folder_time_mode, 0);
    assert_eq!(config.file_attribute, 32);
    assert_eq!(config.folder_attribute, 16);
    assert_eq!(config.warning_level, 2);
    assert_eq!(config.recycles_individual_files, true);
    assert_eq!(config.exits_immediately, true);
    assert_eq!(config.pre_check, false);
    
    // Test regex patterns
    assert!(config.append_file.is_some());
    assert!(config.exclude_file.is_some());
    assert!(config.remarkable_file.is_some());
    
    // Test regex matching
    assert!(config.should_exclude_file("test.tmp"));
    assert!(config.should_exclude_file("backup.bak"));
    assert!(!config.should_exclude_file("document.txt"));
    
    assert!(config.should_include_file("document.txt"));
    assert!(config.should_include_file("file.doc"));
    assert!(!config.should_include_file("image.jpg"));
    
    assert!(config.is_remarkable_file("program.exe"));
    assert!(!config.is_remarkable_file("document.txt"));
}

#[test]
fn test_default_config() {
    let config = Config::default();
    
    assert_eq!(config.priority_class, 0);
    assert_eq!(config.search_subfolders, true);
    assert_eq!(config.deletes_empty_folder, true);
    assert_eq!(config.uses_recycle_bin, true);
    assert_eq!(config.removes_redundant_folder, 1);
    assert_eq!(config.shows_progress, true);
    assert_eq!(config.compress_level, -1);
    assert_eq!(config.batch_decompress, true);
    assert!(config.append_file.is_none());
    assert!(config.exclude_file.is_none());
    assert!(config.remarkable_file.is_none());
}

#[test]
fn test_nonexistent_config_file() {
    let nonexistent_path = Path::new("/nonexistent/config.ini");
    let config = Config::load(nonexistent_path).unwrap();
    
    // Should return default configuration
    assert_eq!(config.priority_class, 0);
    assert_eq!(config.search_subfolders, true);
}

#[test]
fn test_compression_level_conversion() {
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