use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use tempfile::TempDir;
use unify_zip::config::Config;
use unify_zip::processor::ArchiveProcessor;

/// Create a test ZIP file with redundant directory structure and various file types
fn create_test_zip_archive(zip_path: &Path) -> anyhow::Result<()> {
    let file = fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Create a redundant directory structure: wrapper/content/actual_files
    // This structure should be flattened to just content/actual_files

    // Add directory entries
    zip.add_directory("wrapper/", options)?;
    zip.add_directory("wrapper/content/", options)?;
    zip.add_directory("wrapper/content/docs/", options)?;
    zip.add_directory("wrapper/content/src/", options)?;

    // Add files that should be kept
    zip.start_file("wrapper/content/README.md", options)?;
    zip.write_all(b"# Test Project\nThis is a test readme file.")?;

    zip.start_file("wrapper/content/main.rs", options)?;
    zip.write_all(b"fn main() {\n    println!(\"Hello, world!\");\n}")?;

    zip.start_file("wrapper/content/docs/guide.md", options)?;
    zip.write_all(b"# User Guide\nThis is the user guide.")?;

    zip.start_file("wrapper/content/src/lib.rs", options)?;
    zip.write_all(b"// Library code\npub fn hello() -> String {\n    \"Hello\".to_string()\n}")?;

    // Add files that should be filtered out (based on common exclude patterns)
    zip.start_file("wrapper/content/temp.tmp", options)?;
    zip.write_all(b"temporary file content")?;

    zip.start_file("wrapper/content/backup.bak", options)?;
    zip.write_all(b"backup file content")?;

    zip.start_file("wrapper/content/.DS_Store", options)?;
    zip.write_all(b"macOS metadata")?;

    zip.start_file("wrapper/content/Thumbs.db", options)?;
    zip.write_all(b"Windows thumbnail cache")?;

    zip.finish()?;
    Ok(())
}

/// Create a test INI configuration file
fn create_test_config(config_path: &Path) -> anyhow::Result<()> {
    let config_content = r#"[General]
PriorityClass=2
SearchSubfolders=1
ExcludeFile=.*\.(tmp|bak|DS_Store)$|Thumbs\.db$
CreateFolder=0
OverwriteCompress=1
DeleteFiles=1
MoveToRecycleBin=1
SuppressGUI=1

[ZIP]
CompressType=9
PasswordZip=0
EncryptionMethod=0
SFXType=0
"#;
    fs::write(config_path, config_content)?;
    Ok(())
}

#[test]
fn test_complete_archive_processing_workflow() -> anyhow::Result<()> {
    // Initialize logger for debugging
    env_logger::try_init().ok();
    
    // Create temporary directory for testing
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Create test files
    let original_zip = temp_path.join("test_archive.zip");
    let config_file = temp_path.join("UnifyZip.ini");
    let output_dir = temp_path.join("output");
    fs::create_dir_all(&output_dir)?;

    // Create test archive and config
    create_test_zip_archive(&original_zip)?;
    create_test_config(&config_file)?;

    println!("Created test archive: {:?}", original_zip);
    println!("Created config file: {:?}", config_file);

    // Verify original archive exists and has expected size
    assert!(original_zip.exists());
    let original_metadata = fs::metadata(&original_zip)?;
    assert!(original_metadata.len() > 0);
    println!("Original archive size: {} bytes", original_metadata.len());

    // Load configuration
    let config = Config::load(&config_file)?;
    println!("Loaded configuration successfully");

    // Create processor
    let processor = ArchiveProcessor::new(config, false);

    // Copy the original archive to output directory for processing
    let archive_to_process = output_dir.join("test_archive.zip");
    fs::copy(&original_zip, &archive_to_process)?;
    println!("Copied archive to: {:?}", archive_to_process);

    // Process the archive using process_path (public method)
    println!("Starting archive processing...");
    let result = processor.process_path(&archive_to_process)?;
    println!("Processing completed, processed {} files", result);

    // Verify results
    println!("Verifying processing results...");

    // Original archive should be moved to trash (so it shouldn't exist in original location)
    // Note: We can't easily test trash functionality in unit tests, so we'll check if processing succeeded
    
    // Check if a new processed archive was created
    let _expected_output = output_dir.join("test_archive.zip");
    
    // The processor might create a new file with a different name if the original still exists
    let output_files: Vec<PathBuf> = fs::read_dir(&output_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("zip"))
        .collect();

    println!("ZIP files in output directory: {:?}", output_files);
    assert!(!output_files.is_empty(), "No ZIP files found in output directory");

    // Pick the first (or only) ZIP file for verification
    let processed_archive = &output_files[0];
    assert!(processed_archive.exists(), "Processed archive should exist");

    println!("Found processed archive: {:?}", processed_archive);

    // Verify the processed archive contents
    let processed_file = fs::File::open(processed_archive)?;
    let mut processed_zip = zip::ZipArchive::new(processed_file)?;

    println!("Processed archive contains {} entries", processed_zip.len());

    // Collect all file names in the processed archive
    let mut processed_files = Vec::new();
    for i in 0..processed_zip.len() {
        let entry = processed_zip.by_index(i)?;
        processed_files.push(entry.name().to_string());
    }

    println!("Files in processed archive:");
    for file in &processed_files {
        println!("  - {}", file);
    }

    // Verify redundant directory structure was removed
    // Files should now start with "content/" instead of "wrapper/content/"
    let files_with_wrapper: Vec<&String> = processed_files
        .iter()
        .filter(|name| name.starts_with("wrapper/"))
        .collect();
    assert!(files_with_wrapper.is_empty(), 
        "Redundant 'wrapper/' prefix should be removed, but found: {:?}", files_with_wrapper);

    // Verify expected files are present (without redundant prefix)
    let expected_files = [
        "content/README.md",
        "content/main.rs", 
        "content/docs/guide.md",
        "content/src/lib.rs",
    ];

    for expected_file in &expected_files {
        assert!(processed_files.iter().any(|f| f == expected_file || f.ends_with(expected_file)),
            "Expected file '{}' not found in processed archive. Files: {:?}", 
            expected_file, processed_files);
    }

    // Verify filtered files are excluded
    let excluded_patterns = [".tmp", ".bak", ".DS_Store", "Thumbs.db"];
    for pattern in &excluded_patterns {
        let excluded_files: Vec<&String> = processed_files
            .iter()
            .filter(|name| name.contains(pattern))
            .collect();
        assert!(excluded_files.is_empty(),
            "Files with pattern '{}' should be excluded, but found: {:?}", 
            pattern, excluded_files);
    }

    println!("✓ All verification checks passed!");
    println!("✓ Redundant directory structure was removed");
    println!("✓ File filtering worked correctly");
    println!("✓ New archive was created successfully");

    Ok(())
}

#[test]
fn test_archive_with_no_redundant_structure() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let original_zip = temp_path.join("normal_archive.zip");
    let config_file = temp_path.join("UnifyZip.ini");

    // Create ZIP with normal structure (no redundant nesting)
    let file = fs::File::create(&original_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default();

    zip.add_directory("src/", options)?;
    zip.add_directory("docs/", options)?;

    zip.start_file("README.md", options)?;
    zip.write_all(b"# Project\nNormal structure")?;

    zip.start_file("src/main.rs", options)?;
    zip.write_all(b"fn main() {}")?;

    zip.start_file("docs/guide.md", options)?;
    zip.write_all(b"# Guide")?;

    zip.start_file("temp.tmp", options)?;
    zip.write_all(b"temp file")?;

    zip.finish()?;

    // Create simple config that only filters temp files
    create_test_config(&config_file)?;

    let config = Config::load(&config_file)?;
    let processor = ArchiveProcessor::new(config, false);

    let result = processor.process_path(&original_zip)?;
    println!("Processing result for normal structure: {:?}", result);

    // Should still process (filter files) even without redundant structure
    assert!(result > 0); // Should have processed 1 archive

    Ok(())
}

#[test]
fn test_archive_with_deep_redundant_nesting() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let original_zip = temp_path.join("deep_nested_archive.zip");
    let config_file = temp_path.join("UnifyZip.ini");

    // Create ZIP with very deep redundant nesting
    let file = fs::File::create(&original_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default();

    // Create deep structure: level1/level2/level3/level4/actual_content/
    zip.add_directory("level1/", options)?;
    zip.add_directory("level1/level2/", options)?;
    zip.add_directory("level1/level2/level3/", options)?;
    zip.add_directory("level1/level2/level3/level4/", options)?;
    zip.add_directory("level1/level2/level3/level4/actual_content/", options)?;

    zip.start_file("level1/level2/level3/level4/actual_content/important.txt", options)?;
    zip.write_all(b"Important content")?;

    zip.start_file("level1/level2/level3/level4/actual_content/data.csv", options)?;
    zip.write_all(b"col1,col2\nval1,val2")?;

    zip.finish()?;

    create_test_config(&config_file)?;

    let config = Config::load(&config_file)?;
    let processor = ArchiveProcessor::new(config, false);

    let result = processor.process_path(&original_zip)?;

    // Should detect and remove deep redundant nesting
    assert!(result > 0); // Should have processed 1 archive

    println!("Deep nesting test passed: {:?}", result);

    Ok(())
}