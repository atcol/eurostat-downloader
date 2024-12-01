use crate::error::DownloaderError;
use crate::types::InventoryEntry;
use csv::ReaderBuilder;
use log::debug;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn parse_tsv(path: &Path) -> Result<Vec<InventoryEntry>, DownloaderError> {
    let mut file = File::open(path)
        .await
        .map_err(|e| DownloaderError::IoError(e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .await
        .map_err(|e| DownloaderError::IoError(e))?;

    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(contents.as_bytes());

    let mut entries = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| DownloaderError::ParseError(e.to_string()))?;
        
        let entry = InventoryEntry {
            code: record.get(0).unwrap_or("").to_string(),
            entry_type: record.get(1).unwrap_or("").to_string(),
            source_dataset: record.get(2).unwrap_or("").to_string(),
            last_data_change: record.get(3).unwrap_or("").to_string(),
            last_structural_change: record.get(4).unwrap_or("").to_string(),
            tsv_url: Some(record.get(5).unwrap_or("").to_string())
                .filter(|s| !s.is_empty()),
            csv_url: Some(record.get(6).unwrap_or("").to_string())
                .filter(|s| !s.is_empty()),
            sdmx_url: Some(record.get(7).unwrap_or("").to_string())
                .filter(|s| !s.is_empty()),
            structure_url: Some(record.get(8).unwrap_or("").to_string())
                .filter(|s| !s.is_empty()),
            browser_url: Some(record.get(9).unwrap_or("").to_string())
                .filter(|s| !s.is_empty()),
        };

        debug!("Parsed entry: {:?}", entry);
        entries.push(entry);
    }

    Ok(entries)
}
