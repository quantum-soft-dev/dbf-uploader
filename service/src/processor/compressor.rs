// CSV to gzip compressor
use common::error::{ProcessingError, Result};
use crate::processor::ProcessingData;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use tracing::debug;

/// Compress CSV data to gzip format (in memory or from temp file)
///
/// # Arguments
/// * `csv_data` - CSV data (in memory or temp file)
///
/// # Returns
/// Compressed gzip data (in memory or temp file)
pub fn compress_csv_memory(csv_data: ProcessingData) -> Result<ProcessingData> {
    match &csv_data {
        ProcessingData::InMemory(csv_bytes) => {
            debug!(
                "Compressing CSV data in memory ({} KB)",
                csv_bytes.len() / 1024
            );

            // Compress in memory
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(csv_bytes).map_err(|e| {
                ProcessingError::CompressionError(format!("Failed to compress CSV data: {}", e))
            })?;

            let gzip_bytes = encoder.finish().map_err(|e| {
                ProcessingError::CompressionError(format!(
                    "Failed to finalize gzip compression: {}",
                    e
                ))
            })?;

            debug!(
                "Compressed {} KB to {} KB in memory (ratio: {:.1}%)",
                csv_bytes.len() / 1024,
                gzip_bytes.len() / 1024,
                (gzip_bytes.len() as f64 / csv_bytes.len() as f64) * 100.0
            );

            Ok(ProcessingData::InMemory(gzip_bytes))
        }
        ProcessingData::TempFile(csv_path) => {
            debug!("Compressing CSV temp file: {}", csv_path.display());

            // Read from temp file, compress to another temp file
            let csv_file = File::open(csv_path).map_err(ProcessingError::FileReadError)?;
            let mut csv_reader = BufReader::new(csv_file);

            // Create temp output file
            let temp_dir = std::env::temp_dir();
            let temp_filename = format!("dbf_export_{}.csv.gz", uuid::Uuid::new_v4());
            let gzip_path = temp_dir.join(temp_filename);

            let gzip_file = File::create(&gzip_path).map_err(|e| {
                ProcessingError::CompressionError(format!("Failed to create temp gzip file: {}", e))
            })?;

            let buf_writer = BufWriter::new(gzip_file);
            let mut encoder = GzEncoder::new(buf_writer, Compression::default());

            // Read CSV and write to gzip stream
            let mut buffer = vec![0u8; 8192]; // 8KB buffer
            let mut total_bytes = 0;

            loop {
                let bytes_read = csv_reader
                    .read(&mut buffer)
                    .map_err(ProcessingError::FileReadError)?;

                if bytes_read == 0 {
                    break; // EOF
                }

                encoder.write_all(&buffer[..bytes_read]).map_err(|e| {
                    ProcessingError::CompressionError(format!("Failed to write to gzip: {}", e))
                })?;

                total_bytes += bytes_read;
            }

            // Finish compression and flush
            encoder.finish().map_err(|e| {
                ProcessingError::CompressionError(format!("Failed to finalize gzip: {}", e))
            })?;

            let gzip_size = gzip_path.metadata().map(|m| m.len()).unwrap_or(0);

            debug!(
                "Compressed {} KB to {} KB in temp file (ratio: {:.1}%)",
                total_bytes / 1024,
                gzip_size / 1024,
                (gzip_size as f64 / total_bytes as f64) * 100.0
            );

            // CSV temp file will be cleaned up automatically by Drop
            Ok(ProcessingData::TempFile(gzip_path))
        }
    }
}

/// Compress a CSV file to gzip format (legacy function - creates file on disk)
///
/// # Arguments
/// * `csv_path` - Path to the CSV file to compress
/// * `output_name` - Name for the output gzip file (e.g., "data_table.csv.gz")
///
/// # Returns
/// Path to the created gzip file
pub fn compress_csv(csv_path: PathBuf, output_name: String) -> Result<PathBuf> {
    debug!(
        "Compressing CSV file: {} -> {}",
        csv_path.display(),
        output_name
    );

    // Verify CSV file exists
    if !csv_path.exists() {
        return Err(ProcessingError::FileReadError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("CSV file not found: {}", csv_path.display()),
        )));
    }

    // Open CSV file for reading
    let csv_file = File::open(&csv_path).map_err(ProcessingError::FileReadError)?;
    let mut csv_reader = BufReader::new(csv_file);

    // Create output path (in same directory as CSV)
    let output_path = csv_path
        .parent()
        .ok_or_else(|| {
            ProcessingError::CompressionError("Cannot determine parent directory".to_string())
        })?
        .join(&output_name);

    // Create gzip output file
    let output_file = File::create(&output_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::Other
            && e.to_string().contains("No space left on device")
        {
            ProcessingError::DiskFullError(format!("Cannot create gzip file: {}", e))
        } else {
            ProcessingError::FileReadError(e)
        }
    })?;

    let buf_writer = BufWriter::new(output_file);
    let mut encoder = GzEncoder::new(buf_writer, Compression::default());

    // Read CSV and write to gzip stream
    let mut buffer = vec![0u8; 8192]; // 8KB buffer
    let mut total_bytes = 0;

    loop {
        let bytes_read = csv_reader
            .read(&mut buffer)
            .map_err(ProcessingError::FileReadError)?;

        if bytes_read == 0 {
            break; // EOF
        }

        encoder.write_all(&buffer[..bytes_read]).map_err(|e| {
            if e.kind() == std::io::ErrorKind::Other
                && e.to_string().contains("No space left on device")
            {
                ProcessingError::DiskFullError(format!("Disk full while compressing: {}", e))
            } else {
                ProcessingError::CompressionError(format!("Failed to write to gzip: {}", e))
            }
        })?;

        total_bytes += bytes_read;
    }

    // Finish compression and flush
    encoder.finish().map_err(|e| {
        ProcessingError::CompressionError(format!("Failed to finalize gzip: {}", e))
    })?;

    debug!(
        "Compressed {} bytes to {}",
        total_bytes,
        output_path.display()
    );

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::fs;
    use std::io::Read;
    use tempfile::TempDir;

    #[test]
    fn test_compress_csv_basic() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        // Create test CSV file
        let csv_content = "name,age,city\nAlice,30,NYC\nBob,25,LA\n";
        fs::write(&csv_path, csv_content).unwrap();

        // Compress it
        let output_name = "test_output.csv.gz".to_string();
        let result = compress_csv(csv_path.clone(), output_name);
        assert!(result.is_ok());

        let gzip_path = result.unwrap();
        assert!(gzip_path.exists());
        assert!(gzip_path.file_name().unwrap() == "test_output.csv.gz");

        // Verify we can decompress and read the content
        let gzip_file = File::open(&gzip_path).unwrap();
        let mut decoder = GzDecoder::new(gzip_file);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed).unwrap();

        assert_eq!(decompressed, csv_content);
    }

    #[test]
    fn test_compress_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("nonexistent.csv");

        let result = compress_csv(csv_path, "output.csv.gz".to_string());
        assert!(result.is_err());
        match result {
            Err(ProcessingError::FileReadError(_)) => (),
            _ => panic!("Expected FileReadError"),
        }
    }

    #[test]
    fn test_compress_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("large.csv");

        // Create a larger CSV file (>8KB to test buffering)
        let mut csv_content = String::from("id,data,value\n");
        for i in 0..1000 {
            csv_content.push_str(&format!("{},sample_data_{},12345.67\n", i, i));
        }
        fs::write(&csv_path, &csv_content).unwrap();

        // Compress it
        let result = compress_csv(csv_path, "large.csv.gz".to_string());
        assert!(result.is_ok());

        let gzip_path = result.unwrap();

        // Verify decompression
        let gzip_file = File::open(&gzip_path).unwrap();
        let mut decoder = GzDecoder::new(gzip_file);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed).unwrap();

        assert_eq!(decompressed, csv_content);
    }

    #[test]
    fn test_compress_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("empty.csv");

        // Create empty CSV file
        fs::write(&csv_path, "").unwrap();

        // Compress it
        let result = compress_csv(csv_path, "empty.csv.gz".to_string());
        assert!(result.is_ok());

        let gzip_path = result.unwrap();
        assert!(gzip_path.exists());

        // Verify it's a valid gzip file (even if empty)
        let gzip_file = File::open(&gzip_path).unwrap();
        let mut decoder = GzDecoder::new(gzip_file);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed).unwrap();

        assert_eq!(decompressed, "");
    }
}
