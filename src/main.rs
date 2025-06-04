use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use std::io;
use std::path::PathBuf;

mod archive;
mod config;
mod processor;
mod utils;

use config::Config;
use processor::ArchiveProcessor;

#[derive(Parser, Debug)]
#[command(name = "unify_zip")]
#[command(about = "Archive processing utility that extracts, filters, and recompresses archive files")]
#[command(version = "1.0.0")]
struct Args {
    /// Input files or directories to process
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Configuration file path (defaults to unify_zip.ini in executable directory)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run - show what would be processed without actually doing it
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("UnifyZip Rust version starting");

    // Load configuration
    let config_path = args.config.unwrap_or_else(|| {
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .join("unify_zip.ini")
    });

    let config = Config::load(&config_path)?;
    info!("Configuration loaded from: {:?}", config_path);

    // Create processor
    let processor = ArchiveProcessor::new(config, args.dry_run);

    // Process each input
    for input in &args.inputs {
        if !input.exists() {
            warn!("Input path does not exist: {:?}", input);
            continue;
        }

        match processor.process_path(input) {
            Ok(processed_count) => {
                info!("Processed {} archives from: {:?}", processed_count, input);
            }
            Err(e) => {
                error!("Failed to process {:?}: {}", input, e);
            }
        }
    }

    info!("UnifyZip processing completed");
    
    // Wait for user input before closing (especially useful on Windows)
    println!("\nProcessing completed. Press Enter to exit...");
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut input) {
        warn!("Failed to read input: {}", e);
    }
    
    Ok(())
}