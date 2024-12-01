mod cli;
mod downloader;
mod error;
mod parser;
mod types;

use cli::Cli;
use clap::Parser;
use colored::*;
use log::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting TSV downloader");

    let cli = Cli::parse();
    info!("CLI arguments parsed: parallelism={}", cli.parallelism);

    let inventory = match parser::parse_tsv(&cli.input_file).await {
        Ok(inv) => {
            info!("Parsed {} entries from TSV file", inv.len());
            for entry in &inv {
                info!("Entry: {:?}", entry);
            }
            inv
        }
        Err(e) => {
            error!("Failed to parse TSV file: {}", e);
            eprintln!("{}", "Failed to parse TSV file".red());
            return Err(e.into());
        }
    };

    let downloader = downloader::Downloader::new(cli.parallelism, cli.rate_limit);
    match downloader.download_all(inventory, cli.output_dir).await {
        Ok(summary) => {
            println!("\n{}", "Download Summary:".bold());
            println!("Total downloads: {} files", summary.total_downloads);
            println!("Success rate: {:.1}% ({} files)", 
                (summary.successful_downloads as f64 / summary.total_downloads as f64) * 100.0,
                summary.successful_downloads.to_string().green());
            println!("Failure rate: {:.1}% ({} files)", 
                (summary.failed_downloads as f64 / summary.total_downloads as f64) * 100.0,
                summary.failed_downloads.to_string().red());
            println!("Total data transferred: {:.2} MB", summary.total_bytes_downloaded as f64 / 1_048_576.0);
            println!("Total duration: {:.2?}", summary.total_duration);
            println!("Average speed: {:.2} MB/s", 
                (summary.total_bytes_downloaded as f64 / 1_048_576.0) / summary.total_duration.as_secs_f64());

            if summary.successful_downloads > 0 {
                println!("\n{}", "Successful Downloads:".green().bold());
                for report in summary.reports.iter().filter(|r| r.status == types::DownloadStatus::Success) {
                    let filename = report.task.output_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    println!("✓ {} ({:.2} MB in {:.2?}, {:.2} MB/s)", 
                        filename.green(),
                        report.bytes_downloaded as f64 / 1_048_576.0,
                        report.duration,
                        (report.bytes_downloaded as f64 / 1_048_576.0) / report.duration.as_secs_f64());
                }
            }

            if summary.failed_downloads > 0 {
                println!("\n{}", "Failed Downloads:".red().bold());
                for report in summary.reports.iter().filter(|r| r.status == types::DownloadStatus::Failed) {
                    let filename = report.task.output_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    println!("✗ {} - Error: {}", filename.red(), report.error.as_ref().unwrap());
                    println!("  URL: {}", report.task.url);
                }
            }

            if summary.failed_downloads > 0 {
                error!("{} downloads failed", summary.failed_downloads);
                Err("Some downloads failed".into())
            } else {
                println!("\n{}", "All downloads completed successfully".green());
                Ok(())
            }
        }
        Err(e) => {
            error!("Download process failed: {}", e);
            eprintln!("{}", "Download process failed".red());
            Err(e.into())
        }
    }
}
