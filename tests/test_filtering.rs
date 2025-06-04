use std::path::PathBuf;
use std::time::SystemTime;
use unify_zip::archive::ArchiveEntry;
use unify_zip::utils::FileFilter;
use regex::Regex;

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
fn test_exclude_file_patterns() {
    let mut entries = vec![
        create_test_entry("document.txt", false),
        create_test_entry("backup.bak", false),
        create_test_entry("temp.tmp", false),
        create_test_entry("image.jpg", false),
        create_test_entry("script.sh", false),
        create_test_entry("readme.md", false),
        create_test_entry("folder1", true),
        create_test_entry("folder1/file.txt", false),
        create_test_entry("folder1/temp.tmp", false),
    ];

    // Test excluding temporary and backup files
    let exclude_regex = Regex::new(r".*\.(tmp|bak)$").unwrap();
    let filtered_count = FileFilter::filter_entries(&mut entries, None, Some(&exclude_regex));

    assert_eq!(filtered_count, 3); // Should exclude backup.bak, temp.tmp, folder1/temp.tmp
    assert_eq!(entries.len(), 6);

    // Verify the correct files remain
    let remaining_files: Vec<&str> = entries
        .iter()
        .map(|e| e.path.to_str().unwrap())
        .collect();

    assert!(remaining_files.contains(&"document.txt"));
    assert!(remaining_files.contains(&"image.jpg"));
    assert!(remaining_files.contains(&"script.sh"));
    assert!(remaining_files.contains(&"readme.md"));
    assert!(remaining_files.contains(&"folder1"));
    assert!(remaining_files.contains(&"folder1/file.txt"));

    assert!(!remaining_files.contains(&"backup.bak"));
    assert!(!remaining_files.contains(&"temp.tmp"));
    assert!(!remaining_files.contains(&"folder1/temp.tmp"));
}

#[test]
fn test_include_file_patterns() {
    let mut entries = vec![
        create_test_entry("document.txt", false),
        create_test_entry("presentation.ppt", false),
        create_test_entry("image.jpg", false),
        create_test_entry("video.mp4", false),
        create_test_entry("data.csv", false),
        create_test_entry("report.pdf", false),
        create_test_entry("folder1", true),
        create_test_entry("folder1/notes.txt", false),
        create_test_entry("folder1/photo.png", false),
    ];

    // Test including only documents and text files
    let include_regex = Regex::new(r".*\.(txt|pdf|ppt|csv)$").unwrap();
    let filtered_count = FileFilter::filter_entries(&mut entries, Some(&include_regex), None);

    assert_eq!(filtered_count, 4); // Should exclude image.jpg, video.mp4, folder1/photo.png
    assert_eq!(entries.len(), 6); // 5 files + 1 folder

    let remaining_files: Vec<&str> = entries
        .iter()
        .map(|e| e.path.to_str().unwrap())
        .collect();

    // Should include matching files and directories
    assert!(remaining_files.contains(&"document.txt"));
    assert!(remaining_files.contains(&"presentation.ppt"));
    assert!(remaining_files.contains(&"data.csv"));
    assert!(remaining_files.contains(&"report.pdf"));
    assert!(remaining_files.contains(&"folder1"));
    assert!(remaining_files.contains(&"folder1/notes.txt"));

    // Should exclude non-matching files
    assert!(!remaining_files.contains(&"image.jpg"));
    assert!(!remaining_files.contains(&"video.mp4"));
    assert!(!remaining_files.contains(&"folder1/photo.png"));
}

#[test]
fn test_combined_include_exclude_patterns() {
    let mut entries = vec![
        create_test_entry("important.txt", false),
        create_test_entry("temp.txt", false),
        create_test_entry("backup.txt", false),
        create_test_entry("document.pdf", false),
        create_test_entry("temp.pdf", false),
        create_test_entry("image.jpg", false),
    ];

    // Include only txt and pdf files, but exclude anything with "temp" in the name
    let include_regex = Regex::new(r".*\.(txt|pdf)$").unwrap();
    let exclude_regex = Regex::new(r".*temp.*").unwrap();
    
    let filtered_count = FileFilter::filter_entries(&mut entries, Some(&include_regex), Some(&exclude_regex));

    assert_eq!(filtered_count, 3); // Should exclude image.jpg, temp.txt, temp.pdf
    assert_eq!(entries.len(), 3);

    let remaining_files: Vec<&str> = entries
        .iter()
        .map(|e| e.path.to_str().unwrap())
        .collect();

    assert!(remaining_files.contains(&"important.txt"));
    assert!(remaining_files.contains(&"backup.txt"));
    assert!(remaining_files.contains(&"document.pdf"));

    assert!(!remaining_files.contains(&"temp.txt"));
    assert!(!remaining_files.contains(&"temp.pdf"));
    assert!(!remaining_files.contains(&"image.jpg"));
}

#[test]
fn test_remove_empty_directory_entries() {
    let mut entries = vec![
        create_test_entry("folder1", true),
        create_test_entry("folder1/file.txt", false),
        create_test_entry("empty_folder", true), // This folder has no files
        create_test_entry("folder2", true),
        create_test_entry("folder2/subfolder", true),
        create_test_entry("folder2/subfolder/document.pdf", false),
        create_test_entry("another_empty", true), // This folder has no files
    ];

    let removed_count = FileFilter::remove_empty_directory_entries(&mut entries);

    assert_eq!(removed_count, 2); // Should remove empty_folder and another_empty
    assert_eq!(entries.len(), 5);

    let remaining_paths: Vec<&str> = entries
        .iter()
        .map(|e| e.path.to_str().unwrap())
        .collect();

    // Should keep folders that contain files
    assert!(remaining_paths.contains(&"folder1"));
    assert!(remaining_paths.contains(&"folder2"));
    assert!(remaining_paths.contains(&"folder2/subfolder"));

    // Should keep all files
    assert!(remaining_paths.contains(&"folder1/file.txt"));
    assert!(remaining_paths.contains(&"folder2/subfolder/document.pdf"));

    // Should remove empty folders
    assert!(!remaining_paths.contains(&"empty_folder"));
    assert!(!remaining_paths.contains(&"another_empty"));
}

#[test]
fn test_complex_file_structure_filtering() {
    let mut entries = vec![
        create_test_entry("project", true),
        create_test_entry("project/src", true),
        create_test_entry("project/src/main.rs", false),
        create_test_entry("project/src/lib.rs", false),
        create_test_entry("project/target", true),
        create_test_entry("project/target/debug", true),
        create_test_entry("project/target/debug/deps", true),
        create_test_entry("project/target/debug/build.tmp", false),
        create_test_entry("project/docs", true),
        create_test_entry("project/docs/README.md", false),
        create_test_entry("project/tests", true),
        create_test_entry("project/tests/test1.rs", false),
        create_test_entry("project/.git", true),
        create_test_entry("project/.git/config", false),
        create_test_entry("project/Cargo.toml", false),
        create_test_entry("project/backup.bak", false),
    ];

    // Exclude build artifacts and backup files, include only source and config files
    let include_regex = Regex::new(r".*\.(rs|md|toml)$").unwrap();
    let exclude_regex = Regex::new(r".*(target/|\.git/|\.bak$).*").unwrap();

    let filtered_count = FileFilter::filter_entries(&mut entries, Some(&include_regex), Some(&exclude_regex));

    // Should exclude: build.tmp, .git/config, backup.bak (3 files)
    // And also any non-matching extensions would be excluded by include filter
    assert_eq!(filtered_count, 3);

    let remaining_files: Vec<&str> = entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.to_str().unwrap())
        .collect();

    // Should include source and config files
    assert!(remaining_files.contains(&"project/src/main.rs"));
    assert!(remaining_files.contains(&"project/src/lib.rs"));
    assert!(remaining_files.contains(&"project/docs/README.md"));
    assert!(remaining_files.contains(&"project/tests/test1.rs"));
    assert!(remaining_files.contains(&"project/Cargo.toml"));

    // Should exclude build artifacts and backup files
    assert!(!remaining_files.contains(&"project/target/debug/build.tmp"));
    assert!(!remaining_files.contains(&"project/.git/config"));
    assert!(!remaining_files.contains(&"project/backup.bak"));

    // Test empty directory removal
    let empty_dirs_removed = FileFilter::remove_empty_directory_entries(&mut entries);
    
    // target/debug/deps and target/debug should be removed as they no longer contain files
    // target might also be removed
    assert!(empty_dirs_removed > 0);
}