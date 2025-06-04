use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::TempDir;

/// Supported archive formats
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    Rar,
    SevenZip,
    TarGz,
    TarBz2,
    Lzh,
}

/// Information about a file within an archive
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub original_path: Option<PathBuf>, // Original path before redundant dir removal
    pub size: u64,
    pub compressed_size: u64,
    pub modified_time: Option<SystemTime>,
    pub is_dir: bool,
    pub is_encrypted: bool,
}

/// Archive extractor trait
pub trait ArchiveExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>>;
    fn list_entries(&self) -> Result<Vec<ArchiveEntry>>;
    fn get_format(&self) -> ArchiveFormat;
}

/// ZIP archive extractor
pub struct ZipExtractor {
    file_path: PathBuf,
}

impl ZipExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for ZipExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        use zip::ZipArchive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open ZIP file: {:?}", self.file_path))?;
        let mut archive = ZipArchive::new(BufReader::new(file))
            .with_context(|| "Failed to read ZIP archive")?;

        let mut entries = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .with_context(|| format!("Failed to read ZIP entry {}", i))?;

            let entry_path = PathBuf::from(file.name());
            let sanitized_path = sanitize_path(&entry_path);
            let dest_path = dest_dir.join(&sanitized_path);

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            let zip_datetime = file.last_modified();
            let converted_time = zip_datetime_to_system_time(zip_datetime);
            
            debug!("ZIP entry: {:?}, original datetime: {:04}-{:02}-{:02} {:02}:{:02}:{:02}, converted: {:?}", 
                   sanitized_path, 
                   zip_datetime.year(), zip_datetime.month(), zip_datetime.day(),
                   zip_datetime.hour(), zip_datetime.minute(), zip_datetime.second(),
                   converted_time);

            let entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: file.size(),
                compressed_size: file.compressed_size(),
                modified_time: converted_time,
                is_dir: file.is_dir(),
                is_encrypted: false, // ZIP encryption detection is complex, simplified for now
            };

            if file.is_dir() {
                std::fs::create_dir_all(&dest_path)
                    .with_context(|| format!("Failed to create directory: {:?}", dest_path))?;
            } else {
                let mut outfile = File::create(&dest_path)
                    .with_context(|| format!("Failed to create file: {:?}", dest_path))?;
                std::io::copy(&mut file, &mut outfile)
                    .with_context(|| format!("Failed to extract file: {:?}", dest_path))?;
            }

            // Set original file timestamps after extraction
            if let Some(modified_time) = entry.modified_time {
                let file_time = filetime::FileTime::from_system_time(modified_time);
                if let Err(e) = filetime::set_file_mtime(&dest_path, file_time) {
                    warn!("Failed to set file modification time for {:?}: {}", dest_path, e);
                } else {
                    debug!("Set file modification time for {:?} to {:?}", dest_path, modified_time);
                }
            }

            entries.push(entry);
        }

        Ok(entries)
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        use zip::ZipArchive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open ZIP file: {:?}", self.file_path))?;
        let mut archive = ZipArchive::new(BufReader::new(file))
            .with_context(|| "Failed to read ZIP archive")?;

        let mut entries = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)
                .with_context(|| format!("Failed to read ZIP entry {}", i))?;

            let entry_path = PathBuf::from(file.name());
            let sanitized_path = sanitize_path(&entry_path);

            let zip_datetime = file.last_modified();
            let converted_time = zip_datetime_to_system_time(zip_datetime);
            
            debug!("ZIP list entry: {:?}, original datetime: {:04}-{:02}-{:02} {:02}:{:02}:{:02}, converted: {:?}", 
                   sanitized_path, 
                   zip_datetime.year(), zip_datetime.month(), zip_datetime.day(),
                   zip_datetime.hour(), zip_datetime.minute(), zip_datetime.second(),
                   converted_time);

            let entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: file.size(),
                compressed_size: file.compressed_size(),
                modified_time: converted_time,
                is_dir: file.is_dir(),
                is_encrypted: false, // ZIP encryption detection is complex, simplified for now
            };

            entries.push(entry);
        }

        Ok(entries)
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}

/// RAR archive extractor
pub struct RarExtractor {
    file_path: PathBuf,
}

impl RarExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for RarExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        #[cfg(feature = "rar")]
        {
            self.extract_with_rar_crate(dest_dir)
        }
        #[cfg(not(feature = "rar"))]
        {
            Err(anyhow::anyhow!(
                "RAR support not enabled. Build with --features rar to enable RAR extraction."
            ))
        }
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        #[cfg(feature = "rar")]
        {
            self.list_with_rar_crate()
        }
        #[cfg(not(feature = "rar"))]
        {
            Err(anyhow::anyhow!(
                "RAR support not enabled. Build with --features rar to enable RAR listing."
            ))
        }
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::Rar
    }
}

impl RarExtractor {
    #[cfg(feature = "rar")]
    fn extract_with_rar_crate(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        use rar::Archive;
        
        // Extract using rar crate - it has a simplified API
        let file_path_str = self.file_path.to_string_lossy();
        let dest_path_str = dest_dir.to_string_lossy();
        
        let archive = Archive::extract_all(&file_path_str, &dest_path_str, "")
            .map_err(|e| anyhow::anyhow!("Failed to extract RAR archive: {}", e))?;
        
        let mut entries = Vec::new();
        
        // Since rar crate extracts everything, we need to scan the extracted directory
        // to build the entry list
        self.scan_extracted_directory(dest_dir, &mut entries)?;
        
        Ok(entries)
    }
    
    #[cfg(feature = "rar")]
    fn scan_extracted_directory(&self, dir: &Path, entries: &mut Vec<ArchiveEntry>) -> Result<()> {
        use walkdir::WalkDir;
        
        for entry in WalkDir::new(dir).into_iter() {
            let entry = entry.map_err(|e| anyhow::anyhow!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            // Skip the base directory itself
            if path == dir {
                continue;
            }
            
            // Get relative path from dest_dir
            let relative_path = path.strip_prefix(dir)
                .map_err(|e| anyhow::anyhow!("Failed to get relative path: {}", e))?;
            
            let metadata = entry.metadata()
                .map_err(|e| anyhow::anyhow!("Failed to get metadata: {}", e))?;
            
            entries.push(ArchiveEntry {
                path: sanitize_path(relative_path),
                original_path: None,
                size: if metadata.is_dir() { 0 } else { metadata.len() },
                compressed_size: 0, // Not available from extraction
                modified_time: metadata.modified().ok(),
                is_dir: metadata.is_dir(),
                is_encrypted: false, // Assume not encrypted after extraction
            });
        }
        
        Ok(())
    }
    
    #[cfg(feature = "rar")]
    fn list_with_rar_crate(&self) -> Result<Vec<ArchiveEntry>> {
        // rar crate doesn't provide a listing-only API
        // For now, we'll return an error suggesting extraction instead
        Err(anyhow::anyhow!(
            "RAR listing not supported with current rar crate. Use extract instead."
        ))
    }
}

/// 7-Zip archive extractor
pub struct SevenZipExtractor {
    file_path: PathBuf,
}

impl SevenZipExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for SevenZipExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        self.extract_with_sevenz_rust(dest_dir)
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        self.list_with_sevenz_rust()
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZip
    }
}

impl SevenZipExtractor {
    fn extract_with_sevenz_rust(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        // For now, use a simplified approach that relies on external 7z tool
        // since sevenz-rust API is complex and rapidly changing
        Err(anyhow::anyhow!(
            "7-Zip support is temporarily limited. Consider using external 7z tool or converting to ZIP format."
        ))
    }
    
    fn list_with_sevenz_rust(&self) -> Result<Vec<ArchiveEntry>> {
        Err(anyhow::anyhow!(
            "7-Zip listing is temporarily limited."
        ))
    }
}

/// TAR.GZ archive extractor
pub struct TarGzExtractor {
    file_path: PathBuf,
}

impl TarGzExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for TarGzExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open tar.gz file: {:?}", self.file_path))?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        let mut entries = Vec::new();

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let sanitized_path = sanitize_path(&path);
            let dest_path = dest_dir.join(&sanitized_path);

            let archive_entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: entry.size(),
                compressed_size: entry.size(), // TAR doesn't track compressed size per entry
                modified_time: entry.header().mtime().ok().map(|t| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
                is_dir: entry.header().entry_type().is_dir(),
                is_encrypted: false, // TAR.GZ doesn't support per-file encryption
            };

            entry.unpack(&dest_path)
                .with_context(|| format!("Failed to extract tar.gz entry: {:?}", dest_path))?;

            // Set original file timestamps after extraction
            if let Some(modified_time) = archive_entry.modified_time {
                let file_time = filetime::FileTime::from_system_time(modified_time);
                if let Err(e) = filetime::set_file_mtime(&dest_path, file_time) {
                    warn!("Failed to set file modification time for {:?}: {}", dest_path, e);
                } else {
                    debug!("Set file modification time for {:?} to {:?}", dest_path, modified_time);
                }
            }

            entries.push(archive_entry);
        }

        Ok(entries)
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open tar.gz file: {:?}", self.file_path))?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        let mut entries = Vec::new();

        for entry_result in archive.entries()? {
            let entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let sanitized_path = sanitize_path(&path);

            let archive_entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: entry.size(),
                compressed_size: entry.size(),
                modified_time: entry.header().mtime().ok().map(|t| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
                is_dir: entry.header().entry_type().is_dir(),
                is_encrypted: false,
            };

            entries.push(archive_entry);
        }

        Ok(entries)
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::TarGz
    }
}

/// TAR.BZ2 archive extractor
pub struct TarBz2Extractor {
    file_path: PathBuf,
}

impl TarBz2Extractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for TarBz2Extractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        use bzip2::read::BzDecoder;
        use tar::Archive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open tar.bz2 file: {:?}", self.file_path))?;
        let bz = BzDecoder::new(file);
        let mut archive = Archive::new(bz);

        let mut entries = Vec::new();

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let sanitized_path = sanitize_path(&path);
            let dest_path = dest_dir.join(&sanitized_path);

            let archive_entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: entry.size(),
                compressed_size: entry.size(),
                modified_time: entry.header().mtime().ok().map(|t| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
                is_dir: entry.header().entry_type().is_dir(),
                is_encrypted: false,
            };

            entry.unpack(&dest_path)
                .with_context(|| format!("Failed to extract tar.bz2 entry: {:?}", dest_path))?;

            // Set original file timestamps after extraction
            if let Some(modified_time) = archive_entry.modified_time {
                let file_time = filetime::FileTime::from_system_time(modified_time);
                if let Err(e) = filetime::set_file_mtime(&dest_path, file_time) {
                    warn!("Failed to set file modification time for {:?}: {}", dest_path, e);
                } else {
                    debug!("Set file modification time for {:?} to {:?}", dest_path, modified_time);
                }
            }

            entries.push(archive_entry);
        }

        Ok(entries)
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        use bzip2::read::BzDecoder;
        use tar::Archive;

        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open tar.bz2 file: {:?}", self.file_path))?;
        let bz = BzDecoder::new(file);
        let mut archive = Archive::new(bz);

        let mut entries = Vec::new();

        for entry_result in archive.entries()? {
            let entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let sanitized_path = sanitize_path(&path);

            let archive_entry = ArchiveEntry {
                path: sanitized_path,
                original_path: None,
                size: entry.size(),
                compressed_size: entry.size(),
                modified_time: entry.header().mtime().ok().map(|t| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
                is_dir: entry.header().entry_type().is_dir(),
                is_encrypted: false,
            };

            entries.push(archive_entry);
        }

        Ok(entries)
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::TarBz2
    }
}

/// LZH archive extractor
pub struct LzhExtractor {
    file_path: PathBuf,
}

impl LzhExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl ArchiveExtractor for LzhExtractor {
    fn extract_to(&self, dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        #[cfg(feature = "lzh")]
        {
            self.extract_with_delharc(dest_dir)
        }
        #[cfg(not(feature = "lzh"))]
        {
            Err(anyhow::anyhow!(
                "LZH support not enabled. Build with --features lzh to enable LZH extraction."
            ))
        }
    }

    fn list_entries(&self) -> Result<Vec<ArchiveEntry>> {
        #[cfg(feature = "lzh")]
        {
            self.list_with_delharc()
        }
        #[cfg(not(feature = "lzh"))]
        {
            Err(anyhow::anyhow!(
                "LZH support not enabled. Build with --features lzh to enable LZH listing."
            ))
        }
    }

    fn get_format(&self) -> ArchiveFormat {
        ArchiveFormat::Lzh
    }
}

impl LzhExtractor {
    #[cfg(feature = "lzh")]
    fn extract_with_delharc(&self, _dest_dir: &Path) -> Result<Vec<ArchiveEntry>> {
        // delharc API is complex and still evolving
        // For now, provide basic support indication
        Err(anyhow::anyhow!(
            "LZH extraction is currently experimental. Consider converting to ZIP format first."
        ))
    }

    #[cfg(feature = "lzh")]
    fn list_with_delharc(&self) -> Result<Vec<ArchiveEntry>> {
        Err(anyhow::anyhow!(
            "LZH listing is currently experimental."
        ))
    }
}

/// Detect archive format from file extension
pub fn detect_format(path: &Path) -> Option<ArchiveFormat> {
    let extension = path.extension()?.to_str()?.to_lowercase();
    
    match extension.as_str() {
        "zip" => Some(ArchiveFormat::Zip),
        "rar" => Some(ArchiveFormat::Rar),
        "7z" => Some(ArchiveFormat::SevenZip),
        "lzh" | "lha" => Some(ArchiveFormat::Lzh),
        "gz" => {
            // Check for .tar.gz
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.ends_with(".tar") {
                    return Some(ArchiveFormat::TarGz);
                }
            }
            None
        }
        "bz2" => {
            // Check for .tar.bz2
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.ends_with(".tar") {
                    return Some(ArchiveFormat::TarBz2);
                }
            }
            None
        }
        _ => None,
    }
}

/// Create an appropriate extractor for the given archive
pub fn create_extractor(path: PathBuf) -> Result<Box<dyn ArchiveExtractor>> {
    let format = detect_format(&path)
        .ok_or_else(|| anyhow!("Unsupported archive format: {:?}", path))?;

    let extractor: Box<dyn ArchiveExtractor> = match format {
        ArchiveFormat::Zip => Box::new(ZipExtractor::new(path)),
        ArchiveFormat::Rar => Box::new(RarExtractor::new(path)),
        ArchiveFormat::SevenZip => Box::new(SevenZipExtractor::new(path)),
        ArchiveFormat::TarGz => Box::new(TarGzExtractor::new(path)),
        ArchiveFormat::TarBz2 => Box::new(TarBz2Extractor::new(path)),
        ArchiveFormat::Lzh => Box::new(LzhExtractor::new(path)),
    };

    Ok(extractor)
}

/// ZIP archive creator
pub struct ZipCreator {
    temp_dir: TempDir,
    entries: Vec<ArchiveEntry>,
}

impl ZipCreator {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()
            .with_context(|| "Failed to create temporary directory")?;

        Ok(Self {
            temp_dir,
            entries: Vec::new(),
        })
    }

    /// Add a file or directory to the archive
    pub fn add_path(&mut self, src_path: &Path, archive_path: &Path) -> Result<()> {
        if src_path.is_dir() {
            self.add_directory(src_path, archive_path)?;
        } else {
            self.add_file(src_path, archive_path)?;
        }
        Ok(())
    }

    fn add_file(&mut self, src_path: &Path, archive_path: &Path) -> Result<()> {
        let metadata = src_path.metadata()
            .with_context(|| format!("Failed to get metadata for: {:?}", src_path))?;

        let entry = ArchiveEntry {
            path: archive_path.to_path_buf(),
            original_path: None,
            size: metadata.len(),
            compressed_size: 0, // Will be determined during compression
            modified_time: metadata.modified().ok(),
            is_dir: false,
            is_encrypted: false,
        };

        // Copy file to temp directory and preserve timestamps
        let temp_path = self.temp_dir.path().join(archive_path);
        if let Some(parent) = temp_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src_path, &temp_path)?;
        
        // Preserve the original modification time on the temp file
        if let Some(modified_time) = entry.modified_time {
            let file_time = filetime::FileTime::from_system_time(modified_time);
            if let Err(e) = filetime::set_file_mtime(&temp_path, file_time) {
                warn!("Failed to set temp file modification time for {:?}: {}", temp_path, e);
            }
        }

        self.entries.push(entry);
        Ok(())
    }

    fn add_directory(&mut self, src_path: &Path, archive_path: &Path) -> Result<()> {
        let metadata = src_path.metadata()
            .with_context(|| format!("Failed to get metadata for: {:?}", src_path))?;

        let entry = ArchiveEntry {
            path: archive_path.to_path_buf(),
            original_path: None,
            size: 0,
            compressed_size: 0,
            modified_time: metadata.modified().ok(),
            is_dir: true,
            is_encrypted: false,
        };

        // Create directory in temp and preserve timestamps
        let temp_path = self.temp_dir.path().join(archive_path);
        std::fs::create_dir_all(&temp_path)?;
        
        // Preserve the original modification time on the temp directory
        if let Some(modified_time) = entry.modified_time {
            let file_time = filetime::FileTime::from_system_time(modified_time);
            if let Err(e) = filetime::set_file_mtime(&temp_path, file_time) {
                warn!("Failed to set temp directory modification time for {:?}: {}", temp_path, e);
            }
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Create the ZIP archive at the specified path
    pub fn create(&self, output_path: &Path, compression_level: i32) -> Result<()> {
        use zip::write::{FileOptions, ZipWriter};
        use zip::CompressionMethod;

        let file = File::create(output_path)
            .with_context(|| format!("Failed to create ZIP file: {:?}", output_path))?;
        let mut zip = ZipWriter::new(file);

        let compression = match compression_level {
            0 => CompressionMethod::Stored,
            1..=9 => CompressionMethod::Deflated,
            _ => CompressionMethod::Deflated, // Default
        };

        let options = FileOptions::default()
            .compression_method(compression)
            .compression_level(Some(compression_level.clamp(1, 9)));

        // Sort entries by path for consistent ordering
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));

        // Find the newest modification time among all files
        let newest_time = sorted_entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .filter_map(|entry| entry.modified_time)
            .max();

        for entry in &sorted_entries {
            let archive_path_str = entry.path.to_string_lossy();
            
            if entry.is_dir {
                // Add directory entry with timestamp if available
                let dir_path = if archive_path_str.ends_with('/') {
                    archive_path_str.to_string()
                } else {
                    format!("{}/", archive_path_str)
                };
                
                let dir_options = if let Some(mod_time) = entry.modified_time {
                    if let Some(zip_datetime) = system_time_to_zip_datetime(mod_time) {
                        options.last_modified_time(zip_datetime)
                    } else {
                        options
                    }
                } else {
                    options
                };
                
                zip.add_directory(dir_path, dir_options)?;
            } else {
                // Add file entry with original timestamp
                let file_options = if let Some(mod_time) = entry.modified_time {
                    if let Some(zip_datetime) = system_time_to_zip_datetime(mod_time) {
                        options.last_modified_time(zip_datetime)
                    } else {
                        options
                    }
                } else {
                    options
                };
                
                zip.start_file(archive_path_str, file_options)?;
                
                let temp_file_path = self.temp_dir.path().join(&entry.path);
                let mut file = File::open(&temp_file_path)
                    .with_context(|| format!("Failed to open temp file: {:?}", temp_file_path))?;
                
                std::io::copy(&mut file, &mut zip)
                    .with_context(|| format!("Failed to write file to ZIP: {:?}", entry.path))?;
            }
        }

        zip.finish()
            .with_context(|| "Failed to finalize ZIP archive")?;

        // Set the ZIP file modification time to the newest file's time
        if let Some(newest) = newest_time {
            let file_time = filetime::FileTime::from_system_time(newest);
            if let Err(e) = filetime::set_file_mtime(output_path, file_time) {
                warn!("Failed to set ZIP file modification time: {}", e);
            } else {
                debug!("Set ZIP file modification time to: {:?}", newest);
            }
        }

        info!("Created ZIP archive: {:?} with {} entries", output_path, sorted_entries.len());
        Ok(())
    }
}

/// Convert SystemTime to zip::DateTime with proper date calculation
fn system_time_to_zip_datetime(system_time: SystemTime) -> Option<zip::DateTime> {
    use std::time::UNIX_EPOCH;
    use zip::DateTime;
    
    // Convert SystemTime to seconds since Unix epoch
    let duration = system_time.duration_since(UNIX_EPOCH).ok()?;
    let timestamp = duration.as_secs();
    
    // Proper Gregorian calendar calculation
    let days_since_epoch = timestamp / 86400;
    let seconds_in_day = timestamp % 86400;
    
    // Calculate date components using proper calendar arithmetic
    let (year, month, day) = days_since_unix_epoch_to_date(days_since_epoch);
    
    // Calculate time components
    let hour = (seconds_in_day / 3600) as u8;
    let minute = ((seconds_in_day % 3600) / 60) as u8;
    let second = (seconds_in_day % 60) as u8;
    
    // Ensure values are within ZIP DateTime valid ranges
    let year = std::cmp::min(2107, std::cmp::max(1980, year)) as u16;
    let month = std::cmp::min(12, std::cmp::max(1, month));
    let day = std::cmp::min(31, std::cmp::max(1, day));
    let hour = std::cmp::min(23, hour);
    let minute = std::cmp::min(59, minute);
    let second = std::cmp::min(59, second);
    
    DateTime::from_date_and_time(year, month, day, hour, minute, second).ok()
}

/// Convert days since Unix epoch to Gregorian calendar date
fn days_since_unix_epoch_to_date(mut days: u64) -> (u32, u8, u8) {
    // Unix epoch is January 1, 1970
    let mut year = 1970u32;
    
    // Handle years
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    
    // Handle months
    let mut month = 1u8;
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    for &days_in_month in &days_in_months {
        if days >= days_in_month as u64 {
            days -= days_in_month as u64;
            month += 1;
        } else {
            break;
        }
    }
    
    let day = (days + 1) as u8; // Days are 1-based
    
    (year, month, day)
}

/// Check if a year is a leap year
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0)
}

/// Convert ZIP DateTime to SystemTime
fn zip_datetime_to_system_time(datetime: zip::DateTime) -> Option<SystemTime> {
    use std::time::UNIX_EPOCH;
    
    // Get year, month, day, hour, minute, second from ZIP DateTime
    let year = datetime.year() as u32;
    let month = datetime.month() as u32;
    let day = datetime.day() as u32;
    let hour = datetime.hour() as u32;
    let minute = datetime.minute() as u32;
    let second = datetime.second() as u32;
    
    // Validate ranges
    if year < 1970 || year > 2200 || month < 1 || month > 12 || day < 1 || day > 31 {
        warn!("Invalid datetime values: {}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);
        return None;
    }
    
    // Calculate days since Unix epoch (1970-01-01)
    let mut days = 0u64;
    
    // Add days for complete years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    
    // Add days for complete months in the current year
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    for m in 1..month {
        if m <= 12 {
            days += days_in_months[(m - 1) as usize] as u64;
        }
    }
    
    // Add days for the current month (day is 1-based)
    days += (day - 1) as u64;
    
    // Calculate total seconds
    let total_seconds = days * 86400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64;
    
    debug!("Converted {}-{:02}-{:02} {:02}:{:02}:{:02} to {} seconds since epoch", 
           year, month, day, hour, minute, second, total_seconds);
    
    // Convert to SystemTime
    Some(UNIX_EPOCH + std::time::Duration::from_secs(total_seconds))
}

/// Sanitize path to prevent directory traversal attacks
fn sanitize_path(path: &Path) -> PathBuf {
    let mut sanitized = PathBuf::new();
    
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => {
                sanitized.push(name);
            }
            std::path::Component::ParentDir => {
                // Ignore ".." components for security
                warn!("Ignoring parent directory component in archive path: {:?}", path);
            }
            std::path::Component::RootDir => {
                // Ignore root directory
            }
            _ => {
                // Ignore other special components
                debug!("Ignoring special path component: {:?}", component);
            }
        }
    }
    
    sanitized
}