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
fn debug_analyze_redundant_folders() {
    let entries = vec![
        create_test_entry("wrapper", true),
        create_test_entry("wrapper/content", true),
        create_test_entry("wrapper/content/file1.txt", false),
        create_test_entry("wrapper/content/file2.txt", false),
        create_test_entry("wrapper/content/subfolder", true),
        create_test_entry("wrapper/content/subfolder/file3.txt", false),
    ];

    println!("Entries:");
    for entry in &entries {
        println!("  {} (dir: {})", entry.path.display(), entry.is_dir);
    }

    let redundant = PathUtils::analyze_redundant_folders(&entries);
    println!("Redundant result: {:?}", redundant);
    
    // We expect this to find "wrapper" as redundant
    assert_eq!(redundant, Some(PathBuf::from("wrapper")));
}