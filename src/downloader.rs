use crate::error::DownloaderError;
use crate::types::{DownloadTask, DownloadReport, DownloadStatus, DownloadSummary, FileFormat, InventoryEntry};
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, error};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

pub struct Downloader {
    parallelism: usize,
    client: reqwest::Client,
    rate_limit: Option<u64>,
}

impl Downloader {
    pub fn new(parallelism: usize, rate_limit: Option<u64>) -> Self {
        Self {
            parallelism,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    pub async fn download_all(
        &self,
        inventory: Vec<InventoryEntry>,
        output_dir: PathBuf,
    ) -> Result<DownloadSummary, DownloaderError> {
        let m = MultiProgress::new();
        let tasks = self.create_download_tasks(inventory, &output_dir);
        let start_time = Instant::now();
        let mut reports = Vec::new();
        
        let chunks: Vec<_> = tasks.chunks(self.parallelism).map(|c| c.to_vec()).collect();
        
        for chunk in chunks {
            let handles: Vec<_> = chunk
                .into_iter()
                .map(|task| {
                    let client = self.client.clone();
                    let pb = m.add(self.create_progress_bar(&task));
                    self.download_file(client, task, pb)
                })
                .collect();

            let chunk_results = futures::future::join_all(handles).await;
            reports.extend(chunk_results);
        }

        let total_duration = start_time.elapsed();
        let total_downloads = reports.len();
        let successful_downloads = reports.iter()
            .filter(|r| r.status == DownloadStatus::Success)
            .count();
        let failed_downloads = total_downloads - successful_downloads;
        let total_bytes_downloaded: u64 = reports.iter()
            .map(|r| r.bytes_downloaded)
            .sum();

        let summary = DownloadSummary {
            total_downloads,
            successful_downloads,
            failed_downloads,
            total_bytes_downloaded,
            total_duration,
            reports,
        };

        if let Err(e) = self.write_stats_csv(&summary, &output_dir).await {
            error!("Failed to write stats CSV: {}", e);
        }

        Ok(summary)
    }

    async fn download_file(
        &self,
        client: reqwest::Client,
        task: DownloadTask,
        pb: ProgressBar,
    ) -> DownloadReport {
        let start_time = Instant::now();
        info!("Starting download: {}", task.url);
        let mut downloaded: u64 = 0;

        let result = async {
            let resp = client
                .get(&task.url)
                .send()
                .await
                .map_err(|e| DownloaderError::RequestError(e))?;

            if !resp.status().is_success() {
                return Err(DownloaderError::DownloadError(format!(
                    "HTTP error: {} for URL: {}", 
                    resp.status(),
                    task.url
                )));
            }

            let total_size = resp.content_length().unwrap_or(0);
            pb.set_length(total_size);

            let mut file = File::create(&task.output_path)
                .await
                .map_err(|e| DownloaderError::IoError(e))?;

            let mut stream = resp.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| DownloaderError::DownloadError(e.to_string()))?;
                
                if let Some(rate_limit) = self.rate_limit {
                    let chunk_size = chunk.len() as u64;
                    let delay = std::time::Duration::from_secs_f64(
                        chunk_size as f64 / rate_limit as f64
                    );
                    info!("Rate limiting: chunk_size={} bytes, delay={:?}, rate={} bytes/s", 
                        chunk_size, delay, rate_limit);
                    sleep(delay).await;
                }
                
                file.write_all(&chunk)
                    .await
                    .map_err(|e| DownloaderError::IoError(e))?;
                
                downloaded += chunk.len() as u64;
                pb.set_position(downloaded);
                pb.set_message(format!("Downloading: {}", task.output_path.display()));
            }

            Ok(())
        }.await;

        let duration = start_time.elapsed();
        let (status, error) = match result {
            Ok(_) => {
                pb.finish();
                (DownloadStatus::Success, None)
            },
            Err(e) => {
                let error_msg = e.to_string();
                error!("Download failed for {}: {}", task.url, error_msg);
                pb.finish_with_message("Download failed");
                (DownloadStatus::Failed, Some(error_msg))
            }
        };

        DownloadReport {
            task,
            status,
            bytes_downloaded: downloaded,
            duration,
            error,
        }
    }

    fn create_download_tasks(&self, inventory: Vec<InventoryEntry>, output_dir: &Path) -> Vec<DownloadTask> {
        info!("Creating download tasks from {} inventory entries", inventory.len());
        let mut tasks = Vec::new();

        for entry in inventory {
            if let Some(url) = &entry.tsv_url {
                tasks.push(self.create_task(entry.clone(), url, output_dir, FileFormat::TSV));
            }
            if let Some(url) = &entry.csv_url {
                tasks.push(self.create_task(entry.clone(), url, output_dir, FileFormat::CSV));
            }
            if let Some(url) = &entry.sdmx_url {
                tasks.push(self.create_task(entry.clone(), url, output_dir, FileFormat::SDMX));
            }
        }

        info!("Created {} download tasks", tasks.len());
        for task in &tasks {
            info!("Download task: {} -> {}", task.url, task.output_path.display());
        }
        tasks
    }

    fn create_task(
        &self,
        entry: InventoryEntry,
        url: &str,
        output_dir: &Path,
        format: FileFormat,
    ) -> DownloadTask {
        let extension = match format {
            FileFormat::TSV => "tsv",
            FileFormat::CSV => "csv",
            FileFormat::SDMX => "sdmx",
        };

        let filename = format!("{}_{}.{}", entry.code, entry.entry_type, extension);
        let output_path = output_dir.join(filename);

        DownloadTask {
            entry,
            url: url.to_string(),
            output_path,
            format,
        }
    }

    fn create_progress_bar(&self, task: &DownloadTask) -> ProgressBar {
        let pb = ProgressBar::new(0);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_message(task.output_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string());
        pb
    }

    async fn write_stats_csv(&self, summary: &DownloadSummary, output_dir: &Path) -> Result<(), DownloaderError> {
        tokio::fs::create_dir_all(output_dir)
            .await
            .map_err(|e| DownloaderError::IoError(e))?;
            
        let stats_path = output_dir.join("download_stats.csv");
        let mut wtr = csv::WriterBuilder::new()
            .from_path(stats_path)?;
        
        // Write header
        wtr.write_record(&[
            "Filename",
            "Status",
            "Size (MB)",
            "Duration (s)",
            "Speed (MB/s)",
            "URL",
            "Error"
        ])?;

        // Write data for each download
        for report in &summary.reports {
            let filename = report.task.output_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let size_mb = report.bytes_downloaded as f64 / 1_048_576.0;
            let duration_secs = report.duration.as_secs_f64();
            let speed_mbs = size_mb / duration_secs;
            
            wtr.write_record(&[
                filename.to_string(),
                format!("{:?}", report.status),
                format!("{:.2}", size_mb),
                format!("{:.2}", duration_secs),
                format!("{:.2}", speed_mbs),
                report.task.url.clone(),
                report.error.clone().unwrap_or_default()
            ])?;
        }
        
        wtr.flush()?;
        Ok(())
    }
}
