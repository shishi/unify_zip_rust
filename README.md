** fully vibe Coded **

# UnifyZip

A cross-platform archive processing utility written in Rust, ported from the original C++ UnifyZip application. This tool extracts archives, filters contents, removes redundant directory structures, and recompresses everything into optimized ZIP files.

## Features

- **Multi-format Support**: Handles ZIP, RAR, 7-Zip, TAR.GZ, TAR.BZ2, and LZH archives
- **Intelligent Processing**: Automatically detects and removes redundant directory nesting
- **Flexible Filtering**: Include/exclude files using regular expressions
- **Cross-platform**: Works on Windows, macOS, and Linux with no external dependencies
- **Safe Operations**: Uses recycle bin/trash for deleted files
- **Configurable**: Extensive configuration options via INI file
- **Unicode Support**: Proper handling of international filenames

## Installation

```bash
cargo build --release
```

The executable will be created at `target/release/unify_zip`.

## Usage

### Basic Usage

```bash
# Process a single archive
unify_zip archive.rar

# Process multiple archives
unify_zip file1.zip file2.7z file3.tar.gz

# Process all archives in a directory (recursively)
unify_zip /path/to/directory

# Dry run (see what would be processed without actually doing it)
unify_zip --dry-run archive.rar

# Use custom configuration file
unify_zip --config custom.ini archive.rar

# Enable verbose logging
unify_zip --verbose archive.rar
```

### Command Line Options

- `-c, --config <PATH>`: Path to configuration file (default: `unify_zip.ini` in executable directory)
- `-v, --verbose`: Enable verbose logging
- `-n, --dry-run`: Show what would be processed without actually doing it
- `-h, --help`: Show help information

## Processing Flow

1. **Extract Archive**: Extracts the input archive to a temporary directory
2. **Filter Files**: Applies include/exclude patterns based on regex configuration
3. **Remove Redundant Directories**: Eliminates unnecessary nested folder structures
4. **Recompress**: Creates a new ZIP archive with optimized structure
5. **Clean Up**: Moves original archive to recycle bin and places new archive in same location

## Configuration

The application is configured via an INI file (`unify_zip.ini` by default). Key settings include:

### File Filtering

```ini
; Include only specific file types
AppendFile=.*\.(txt|doc|pdf|jpg|png)

; Exclude temporary and backup files
ExcludeFile=.*\.(tmp|bak|~|\.DS_Store)
```

### Directory Processing

```ini
; Remove redundant nested directories
RemovesRedundantFolder=1

; Delete empty directories
DeletesEmptyFolder=1

; Search subdirectories recursively
SearchSubfolders=1
```

### Compression Settings

```ini
; Compression level (-1=default, 0=store, 1=fast, 9=best)
CompressLevel=6

; Use batch extraction for better performance
BatchDecompress=1
```

### Safety Options

```ini
; Use recycle bin instead of permanent deletion
UsesRecycleBin=1

; Check if archives are already processed
PreCheck=1
```

## Supported Archive Formats

### Input Formats (Extraction)
- ZIP (`.zip`)
- RAR (`.rar`)
- 7-Zip (`.7z`)
- TAR.GZ (`.tar.gz`)
- TAR.BZ2 (`.tar.bz2`)
- LZH/LHA (`.lzh`, `.lha`)

### Output Format
- ZIP (`.zip`) - All processed archives are recompressed as ZIP files

## Examples

### Remove Temporary Files

```ini
ExcludeFile=.*\.(tmp|temp|bak|~)$
```

### Keep Only Documents

```ini
AppendFile=.*\.(pdf|doc|docx|txt|rtf)$
```

### Process Executable Archives Carefully

```ini
RemarkableFile=.*\.(exe|bat|cmd|msi)$
WarningLevel=2
```

## Building from Source

### Prerequisites

- Rust 1.70 or later
- Cargo (included with Rust)

### Build

```bash
git clone <repository_url>
cd unify_zip
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Platform-Specific Notes

### Windows
- Supports Windows recycle bin functionality
- Handles Windows path separators correctly

### macOS/Linux
- Uses system trash functionality
- Preserves Unix file permissions where possible

## Unicode and Encoding Support

The application properly handles:
- UTF-8 filenames
- International characters
- Various text encodings in archive file names
- Cross-platform path separators

## Performance Considerations

- **Batch Processing**: Enabled by default for better performance on large archives
- **Memory Usage**: Uses memory-mapped files where possible to reduce memory footprint
- **Temporary Files**: All extraction happens in temporary directories that are automatically cleaned up

## Safety Features

- **Dry Run Mode**: Test processing without making changes
- **Recycle Bin**: Safely dispose of original archives
- **Path Sanitization**: Prevents directory traversal attacks from malicious archives
- **Error Handling**: Robust error handling with detailed logging

## Troubleshooting

### Common Issues

1. **Permission Errors**: Ensure you have read/write access to the target directories
2. **Unsupported Archives**: Check that the archive format is supported
3. **Configuration Errors**: Verify regex patterns in the INI file are valid
4. **Path Length Limits**: Some platforms have path length restrictions

### Logging

Use the `--verbose` flag to get detailed information about the processing steps:

```bash
unify_zip --verbose archive.rar
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Original Project

This is a Rust port of the original UnifyZip C++ application. The original project provided the inspiration and feature set for this implementation.
