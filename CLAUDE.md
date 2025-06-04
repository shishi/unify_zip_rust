# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

UnifyZip is a Windows command-line archive utility written in C++ that processes ZIP, LZH, and RAR files. It can extract, compress, and manipulate archives while providing various filtering and transformation options. The original project is located in `original_project/UnifyZip/`.

## Build System

**Visual Studio Project:**
- Solution file: `UnifyZip.sln` (Visual Studio 2013 format)
- Project file: `UnifyZip.vcxproj`
- Target platform: Win32
- Configurations: Debug and Release
- Output: `UnifyZip.exe` in Debug/ or Release/ folders

**Build Commands:**
```bash
# Build with MSBuild (if on Windows with Visual Studio tools)
msbuild UnifyZip.sln /p:Configuration=Release /p:Platform=Win32
msbuild UnifyZip.sln /p:Configuration=Debug /p:Platform=Win32
```

**Dependencies:**
- Boost 1.55.0 (regex library)
- zlib 1.2.8
- Archive DLLs: 7-zip32.dll, UNLHA32.DLL, unrar32.dll, UNZIP32.DLL

## Architecture

**Core Components:**

1. **ArchiveDll System** (`ArchiveDll.h/cpp`): 
   - Abstraction layer for different archive format DLLs (7-zip, UNLHA, UNRAR, UNZIP)
   - `ArchiveDllManager` manages multiple archive handlers
   - Each DLL handles specific archive formats

2. **Zip Processing** (`Zip.h/cpp`):
   - Native ZIP file reading/writing implementation
   - `ZipReader` for reading ZIP archives with memory mapping
   - `ZipWriter` for creating ZIP files
   - Supports detailed ZIP metadata when `USE_ZIP_DETAIL_INFO` is defined

3. **Configuration** (`Profile.h/cpp`):
   - INI-based configuration system (`UnifyZip.ini`)
   - Settings for compression levels, file filtering, batch operations

4. **String/Path Utilities** (`String.h/cpp`, `PathFunc.h/cpp`, `StrFunc.h/cpp`):
   - Custom string handling classes and path manipulation functions
   - Handles Japanese character encoding (Shift-JIS)

5. **Command Line Processing** (`CommandLine.h/cpp`):
   - Parses command-line arguments and file paths

**Key Features:**
- Batch decompression with redundant folder removal
- Regular expression-based file filtering (include/exclude patterns)
- File attribute and timestamp manipulation
- Progress display and warning systems
- Memory-mapped file access for performance

## Configuration Options (UnifyZip.ini)

Key settings include:
- `BatchDecompress`: Enable batch processing mode
- `SearchSubfolders`: Process subdirectories
- `RemovesRedundantFolder`: Remove unnecessary folder nesting
- `CompressLevel`: Compression level (-1 to 9)
- `AppendFile`/`ExcludeFile`: Regex patterns for file filtering
- `FileTimeMode`/`FolderTimeMode`: Timestamp handling

## Development Notes

- Codebase uses Boost libraries extensively
- Memory management uses Boost shared_ptr
- String handling supports multi-byte character sets
- Archive operations are abstracted through DLL interfaces
- Heavy use of Windows APIs for file system operations