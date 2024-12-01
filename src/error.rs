use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloaderError {
    #[error("Failed to parse TSV file: {0}")]
    ParseError(String),

    #[error("Download failed: {0}")]
    DownloadError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
}
