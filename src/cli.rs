use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Input TSV file path
    #[arg(short, long)]
    pub input_file: PathBuf,

    /// Output directory for downloaded files
    #[arg(short, long)]
    pub output_dir: PathBuf,

    /// Number of concurrent downloads
    #[arg(short, long, default_value = "4")]
    pub parallelism: usize,

    /// Download rate limit in bytes per second (optional)
    #[arg(short = 'r', long, default_value = None)]
    pub rate_limit: Option<u64>,
}
