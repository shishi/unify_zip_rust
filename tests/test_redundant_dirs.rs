use std::path::PathBuf;
use std::time::SystemTime;
use unify_zip::archive::ArchiveEntry;
use unify_zip::utils::PathUtils;

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
fn test_analyze_redundant_folders_simple_case() {
    let entries = vec![
        create_test_entry("wrapper", true),
        create_test_entry("wrapper/content", true),
        create_test_entry("wrapper/content/file1.txt", false),
        create_test_entry("wrapper/content/file2.txt", false),
        create_test_entry("wrapper/content/subfolder", true),
        create_test_entry("wrapper/content/subfolder/file3.txt", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, Some(PathBuf::from("wrapper")));
}

#[test]
fn test_analyze_redundant_folders_deep_nesting() {
    let entries = vec![
        create_test_entry("level1", true),
        create_test_entry("level1/level2", true),
        create_test_entry("level1/level2/level3", true),
        create_test_entry("level1/level2/level3/level4", true),
        create_test_entry("level1/level2/level3/level4/actual_content", true),
        create_test_entry("level1/level2/level3/level4/actual_content/file1.txt", false),
        create_test_entry("level1/level2/level3/level4/actual_content/file2.pdf", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    // Should detect that we can remove the first 4 levels
    assert_eq!(redundant, Some(PathBuf::from("level1")));
}

#[test]
fn test_analyze_redundant_folders_no_redundancy() {
    let entries = vec![
        create_test_entry("folder1", true),
        create_test_entry("folder1/file1.txt", false),
        create_test_entry("folder2", true),
        create_test_entry("folder2/file2.txt", false),
        create_test_entry("file_at_root.txt", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, None);
}

#[test]
fn test_analyze_redundant_folders_multiple_branches() {
    let entries = vec![
        create_test_entry("root", true),
        create_test_entry("root/branch1", true),
        create_test_entry("root/branch1/file1.txt", false),
        create_test_entry("root/branch2", true),
        create_test_entry("root/branch2/file2.txt", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    // Since root has multiple child directories with files, it shouldn't be considered redundant
    assert_eq!(redundant, None);
}

#[test]
fn test_analyze_redundant_folders_files_at_intermediate_level() {
    let entries = vec![
        create_test_entry("wrapper", true),
        create_test_entry("wrapper/intermediate_file.txt", false), // File at intermediate level
        create_test_entry("wrapper/content", true),
        create_test_entry("wrapper/content/file1.txt", false),
        create_test_entry("wrapper/content/file2.txt", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    // Since there's a file at the wrapper level, it's not redundant
    assert_eq!(redundant, None);
}

#[test]
fn test_remove_redundant_prefix() {
    let mut entries = vec![
        create_test_entry("redundant/actual", true),
        create_test_entry("redundant/actual/content", true),
        create_test_entry("redundant/actual/content/file1.txt", false),
        create_test_entry("redundant/actual/content/file2.pdf", false),
        create_test_entry("redundant/actual/content/subfolder", true),
        create_test_entry("redundant/actual/content/subfolder/file3.doc", false),
    ];

    let redundant_prefix = PathBuf::from("redundant/actual");
    PathUtils::remove_redundant_prefix(&mut entries, &redundant_prefix).unwrap();

    // Check that paths have been updated correctly
    let expected_paths = vec![
        "content",
        "content/file1.txt",
        "content/file2.pdf",
        "content/subfolder",
        "content/subfolder/file3.doc",
    ];

    let actual_paths: Vec<String> = entries
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    for expected in expected_paths {
        assert!(
            actual_paths.contains(&expected.to_string()),
            "Expected path '{}' not found in result: {:?}",
            expected,
            actual_paths
        );
    }

    assert_eq!(entries.len(), 5); // Should have removed the "content" directory entry since it's now at root
}

#[test]
fn test_complex_redundant_structure() {
    let entries = vec![
        create_test_entry("app-v1.2.3", true),
        create_test_entry("app-v1.2.3/dist", true),
        create_test_entry("app-v1.2.3/dist/src", true),
        create_test_entry("app-v1.2.3/dist/src/main.js", false),
        create_test_entry("app-v1.2.3/dist/src/style.css", false),
        create_test_entry("app-v1.2.3/dist/src/assets", true),
        create_test_entry("app-v1.2.3/dist/src/assets/logo.png", false),
        create_test_entry("app-v1.2.3/dist/src/assets/font.woff", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, Some(PathBuf::from("app-v1.2.3")));

    // Test the full process
    let mut entries_copy = entries.clone();
    if let Some(redundant_path) = redundant {
        PathUtils::remove_redundant_prefix(&mut entries_copy, &redundant_path).unwrap();
    }

    // After removing redundant prefix, the structure should be flattened
    let final_paths: Vec<String> = entries_copy
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    // Should start with "dist" now
    assert!(final_paths.contains(&"dist".to_string()));
    assert!(final_paths.contains(&"dist/src".to_string()));
    assert!(final_paths.contains(&"dist/src/main.js".to_string()));
    assert!(final_paths.contains(&"dist/src/style.css".to_string()));
    assert!(final_paths.contains(&"dist/src/assets".to_string()));
    assert!(final_paths.contains(&"dist/src/assets/logo.png".to_string()));
    assert!(final_paths.contains(&"dist/src/assets/font.woff".to_string()));

    // Should not contain the original redundant prefix
    assert!(!final_paths.iter().any(|p| p.starts_with("app-v1.2.3")));
}

#[test]
fn test_edge_case_single_file_in_deep_structure() {
    let entries = vec![
        create_test_entry("very", true),
        create_test_entry("very/deep", true),
        create_test_entry("very/deep/nested", true),
        create_test_entry("very/deep/nested/structure", true),
        create_test_entry("very/deep/nested/structure/single_file.txt", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, Some(PathBuf::from("very")));

    let mut entries_copy = entries.clone();
    if let Some(redundant_path) = redundant {
        PathUtils::remove_redundant_prefix(&mut entries_copy, &redundant_path).unwrap();
    }

    // Should end up with just the file at the root and necessary parent directories
    let final_paths: Vec<String> = entries_copy
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    assert!(final_paths.contains(&"deep".to_string()));
    assert!(final_paths.contains(&"deep/nested".to_string()));
    assert!(final_paths.contains(&"deep/nested/structure".to_string()));
    assert!(final_paths.contains(&"deep/nested/structure/single_file.txt".to_string()));
}

#[test]
fn test_no_redundancy_with_mixed_content() {
    let entries = vec![
        create_test_entry("project", true),
        create_test_entry("project/README.md", false), // File at project level
        create_test_entry("project/src", true),
        create_test_entry("project/src/main.rs", false),
        create_test_entry("project/tests", true),
        create_test_entry("project/tests/test1.rs", false),
        create_test_entry("project/docs", true),
        create_test_entry("project/docs/guide.md", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    // Since project directory has files and multiple subdirectories, it's not redundant
    assert_eq!(redundant, None);
}

#[test]
fn test_empty_entries_list() {
    let entries = vec![];
    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, None);
}

#[test]
fn test_only_files_no_directories() {
    let entries = vec![
        create_test_entry("file1.txt", false),
        create_test_entry("file2.pdf", false),
        create_test_entry("file3.jpg", false),
    ];

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    assert_eq!(redundant, None);
}