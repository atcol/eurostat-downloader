use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct InventoryEntry {
    pub code: String,
    pub entry_type: String,
    pub source_dataset: String,
    pub last_data_change: String,
    pub last_structural_change: String,
    pub tsv_url: Option<String>,
    pub csv_url: Option<String>,
    pub sdmx_url: Option<String>,
    pub structure_url: Option<String>,
    pub browser_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub entry: InventoryEntry,
    pub url: String,
    pub output_path: PathBuf,
    pub format: FileFormat,
}

#[derive(Debug, Clone)]
pub enum FileFormat {
    TSV,
    CSV,
    SDMX,
}

#[derive(Debug)]
pub struct DownloadReport {
    pub task: DownloadTask,
    pub status: DownloadStatus,
    pub bytes_downloaded: u64,
    pub duration: Duration,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum DownloadStatus {
    Success,
    Failed,
}

#[derive(Debug)]
pub struct DownloadSummary {
    pub total_downloads: usize,
    pub successful_downloads: usize,
    pub failed_downloads: usize,
    pub total_bytes_downloaded: u64,
    pub total_duration: Duration,
    pub reports: Vec<DownloadReport>,
}
