// DBF to CSV converter
use common::error::{ProcessingError, Result};
use common::file_utils::open_shared_read;
use common::models::{Config, DbfFile, Encoding};
use crate::processor::ProcessingData;
use csv::Writer;
use dbase::encoding::LossyCodePage;
use dbase::yore::code_pages::{CP1251, CP1255, CP866};
use dbase::{FieldValue, Record, Unicode};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use tracing::{debug, info, warn};

// Maximum size for in-memory CSV (10 MB)
const MAX_IN_MEMORY_SIZE: usize = 10 * 1024 * 1024;

/// Convert a DBF file to CSV in memory (or temp file if too large)
/// Returns CSV data either in memory or as a temp file path
pub fn convert_dbf_to_csv_memory(dbf_file: &DbfFile, config: &Config) -> Result<ProcessingData> {
    debug!(
        "Converting DBF to CSV (in-memory): {}",
        dbf_file.path.display()
    );

    // Dispatch to the appropriate encoding handler
    match get_encoding_enum(dbf_file.encoding.as_ref(), &config.encoding.dbf_encoding) {
        Encoding::CP866 => convert_with_encoding(dbf_file, LossyCodePage(CP866)),
        Encoding::Windows1251 => convert_with_encoding(dbf_file, LossyCodePage(CP1251)),
        Encoding::Windows1255 => convert_with_encoding(dbf_file, LossyCodePage(CP1255)),
        Encoding::ISO8859_8 => {
            // ISO-8859-8 is not in yore, fall back to CP1255 (Windows Hebrew)
            warn!("ISO-8859-8 not available, using CP1255 (Windows-1255) as fallback");
            convert_with_encoding(dbf_file, LossyCodePage(CP1255))
        }
        Encoding::UTF8 => convert_with_encoding(dbf_file, Unicode),
    }
}

/// Generic function to convert DBF to CSV with a specific encoding
fn convert_with_encoding<E: dbase::Encoding + 'static>(
    dbf_file: &DbfFile,
    encoding: E,
) -> Result<ProcessingData> {
    // Open DBF file with shared read access (allows reading files opened by other processes)
    let file = open_shared_read(&dbf_file.path).map_err(ProcessingError::FileReadError)?;

    let mut reader = dbase::Reader::new_with_encoding(BufReader::new(file), encoding)
        .map_err(|e| ProcessingError::ConversionError(format!("Failed to open DBF file: {}", e)))?;

    // Get field names from DBF header
    let field_names: Vec<String> = reader
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect();

    // Create CSV writer to in-memory buffer
    let mut csv_buffer = Vec::new();
    let mut csv_writer = Writer::from_writer(&mut csv_buffer);

    // Write CSV header
    csv_writer.write_record(&field_names).map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to write CSV header: {}", e))
    })?;

    // Process each record
    let mut record_count = 0;
    let mut estimated_size = 0usize;

    for result in reader.iter_records() {
        match result {
            Ok(record) => {
                let csv_record = convert_record_to_csv(&record, &field_names)?;

                // Estimate size before writing
                let record_size: usize = csv_record.iter().map(|s| s.len() + 1).sum(); // +1 for delimiter/newline
                estimated_size += record_size;

                csv_writer.write_record(&csv_record).map_err(|e| {
                    ProcessingError::ConversionError(format!("Failed to write CSV record: {}", e))
                })?;
                record_count += 1;

                // Check if estimated buffer size is getting too large
                if estimated_size > MAX_IN_MEMORY_SIZE {
                    info!(
                        file = %dbf_file.path.display(),
                        size_mb = estimated_size / 1024 / 1024,
                        "CSV data exceeds {} MB, switching to temp file",
                        MAX_IN_MEMORY_SIZE / 1024 / 1024
                    );

                    // Flush and drop writer before moving buffer
                    csv_writer.flush().map_err(|e| {
                        ProcessingError::ConversionError(format!(
                            "Failed to flush CSV writer: {}",
                            e
                        ))
                    })?;
                    drop(csv_writer);

                    // Fallback to temp file
                    return write_to_temp_file(
                        dbf_file,
                        reader,
                        &field_names,
                        record_count,
                        csv_buffer,
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Skipping corrupted record in {}: {}",
                    dbf_file.path.display(),
                    e
                );
                continue;
            }
        }
    }

    // Flush and finish writing
    csv_writer.flush().map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to flush CSV writer: {}", e))
    })?;

    // Drop writer to release mutable borrow
    drop(csv_writer);

    debug!(
        "Converted {} records from DBF to CSV in memory ({} KB)",
        record_count,
        csv_buffer.len() / 1024
    );

    Ok(ProcessingData::InMemory(csv_buffer))
}

/// Fallback: Write remaining data to temp file when in-memory buffer is too large
fn write_to_temp_file(
    dbf_file: &DbfFile,
    mut reader: dbase::Reader<BufReader<File>>,
    field_names: &[String],
    mut record_count: usize,
    existing_buffer: Vec<u8>,
) -> Result<ProcessingData> {
    // Create temp file in system temp directory
    let temp_dir = std::env::temp_dir();
    let temp_filename = format!("dbf_export_{}.csv", uuid::Uuid::new_v4());
    let csv_path = temp_dir.join(temp_filename);

    debug!("Creating temp CSV file: {}", csv_path.display());

    let csv_file = File::create(&csv_path).map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to create temp CSV file: {}", e))
    })?;

    let mut buf_writer = BufWriter::new(csv_file);

    // Write existing buffer to file
    buf_writer.write_all(&existing_buffer).map_err(|e| {
        ProcessingError::ConversionError(format!(
            "Failed to write buffered data to temp file: {}",
            e
        ))
    })?;

    let mut csv_writer = Writer::from_writer(buf_writer);

    // Continue processing remaining records
    for result in reader.iter_records() {
        match result {
            Ok(record) => {
                let csv_record = convert_record_to_csv(&record, field_names)?;
                csv_writer.write_record(&csv_record).map_err(|e| {
                    ProcessingError::ConversionError(format!("Failed to write CSV record: {}", e))
                })?;
                record_count += 1;
            }
            Err(e) => {
                warn!(
                    "Skipping corrupted record in {}: {}",
                    dbf_file.path.display(),
                    e
                );
                continue;
            }
        }
    }

    csv_writer.flush().map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to flush CSV writer: {}", e))
    })?;

    info!(
        "Converted {} records from DBF to temp CSV file: {} ({} KB)",
        record_count,
        csv_path.display(),
        csv_path.metadata().map(|m| m.len() / 1024).unwrap_or(0)
    );

    Ok(ProcessingData::TempFile(csv_path))
}

/// Convert a DBF file to CSV with UTF-8 encoding (legacy function - creates file on disk)
/// Returns the path to the created CSV file
pub fn convert_dbf_to_csv(dbf_file: &DbfFile, config: &Config) -> Result<PathBuf> {
    debug!("Converting DBF to CSV: {}", dbf_file.path.display());

    // Dispatch to the appropriate encoding handler
    match get_encoding_enum(dbf_file.encoding.as_ref(), &config.encoding.dbf_encoding) {
        Encoding::CP866 => convert_to_file_with_encoding(dbf_file, LossyCodePage(CP866)),
        Encoding::Windows1251 => convert_to_file_with_encoding(dbf_file, LossyCodePage(CP1251)),
        Encoding::Windows1255 => convert_to_file_with_encoding(dbf_file, LossyCodePage(CP1255)),
        Encoding::ISO8859_8 => {
            warn!("ISO-8859-8 not available, using CP1255 (Windows-1255) as fallback");
            convert_to_file_with_encoding(dbf_file, LossyCodePage(CP1255))
        }
        Encoding::UTF8 => convert_to_file_with_encoding(dbf_file, Unicode),
    }
}

/// Generic function to convert DBF to CSV file with a specific encoding
fn convert_to_file_with_encoding<E: dbase::Encoding + 'static>(
    dbf_file: &DbfFile,
    encoding: E,
) -> Result<PathBuf> {
    // Open DBF file with shared read access (allows reading files opened by other processes)
    let file = open_shared_read(&dbf_file.path).map_err(ProcessingError::FileReadError)?;

    let mut reader = dbase::Reader::new_with_encoding(BufReader::new(file), encoding)
        .map_err(|e| ProcessingError::ConversionError(format!("Failed to open DBF file: {}", e)))?;

    // Get field names from DBF header
    let field_names: Vec<String> = reader
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect();

    // Create CSV output file (same directory as DBF, with .csv extension)
    let csv_path = dbf_file.path.with_extension("csv");
    let csv_file = File::create(&csv_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::Other
            && e.to_string().contains("No space left on device")
        {
            ProcessingError::DiskFullError(format!("Cannot create CSV file: {}", e))
        } else {
            ProcessingError::FileReadError(e)
        }
    })?;

    let mut csv_writer = Writer::from_writer(BufWriter::new(csv_file));

    // Write CSV header
    csv_writer.write_record(&field_names).map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to write CSV header: {}", e))
    })?;

    // Process each record
    let mut record_count = 0;
    for result in reader.iter_records() {
        match result {
            Ok(record) => {
                let csv_record = convert_record_to_csv(&record, &field_names)?;
                csv_writer.write_record(&csv_record).map_err(|e| {
                    ProcessingError::ConversionError(format!("Failed to write CSV record: {}", e))
                })?;
                record_count += 1;
            }
            Err(e) => {
                warn!(
                    "Skipping corrupted record in {}: {}",
                    dbf_file.path.display(),
                    e
                );
                // Continue processing other records instead of failing
                continue;
            }
        }
    }

    // Flush and finish writing
    csv_writer.flush().map_err(|e| {
        ProcessingError::ConversionError(format!("Failed to flush CSV writer: {}", e))
    })?;

    debug!(
        "Converted {} records from DBF to CSV: {}",
        record_count,
        csv_path.display()
    );

    Ok(csv_path)
}

/// Get the encoding enum to use for DBF text fields
fn get_encoding_enum(dbf_encoding: Option<&Encoding>, config_encoding: &str) -> Encoding {
    match dbf_encoding {
        Some(enc) => enc.clone(),
        None => {
            // Use config fallback encoding
            match config_encoding.to_uppercase().as_str() {
                "CP866" | "IBM866" => Encoding::CP866,
                "WINDOWS-1251" | "WINDOWS1251" | "CP1251" => Encoding::Windows1251,
                "WINDOWS-1255" | "WINDOWS1255" | "CP1255" => Encoding::Windows1255,
                "ISO-8859-8" | "ISO88598" | "ISO8859-8" | "ISO8859_8" => Encoding::ISO8859_8,
                "UTF-8" | "UTF8" => Encoding::UTF8,
                _ => {
                    warn!(
                        "Unknown encoding '{}', defaulting to Windows-1255 (Hebrew)",
                        config_encoding
                    );
                    Encoding::Windows1255
                }
            }
        }
    }
}

/// Convert a DBF record to CSV string fields
/// Takes field names to ensure proper ordering (Record is a HashMap)
/// Note: The dbase Reader already handles encoding conversion, so strings are already UTF-8
fn convert_record_to_csv(record: &Record, field_names: &[String]) -> Result<Vec<String>> {
    let mut csv_fields = Vec::new();

    // Iterate through field names in order to maintain column order
    for field_name in field_names {
        let field_value = record.get(field_name).ok_or_else(|| {
            ProcessingError::ConversionError(format!("Missing field '{}' in record", field_name))
        })?;

        let field_str = match field_value {
            FieldValue::Character(Some(s)) => s.clone(),
            FieldValue::Character(None) => String::new(),
            FieldValue::Numeric(Some(n)) => n.to_string(),
            FieldValue::Numeric(None) => String::new(),
            FieldValue::Logical(Some(b)) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            FieldValue::Logical(None) => String::new(),
            FieldValue::Date(Some(d)) => {
                // Format as YYYY-MM-DD (date only, no time)
                format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
            }
            FieldValue::Date(None) => String::new(),
            FieldValue::Float(Some(f)) => f.to_string(),
            FieldValue::Float(None) => String::new(),
            FieldValue::Integer(i) => i.to_string(),
            FieldValue::Currency(c) => format!("{:.2}", c / 10000.0),
            FieldValue::DateTime(dt) => {
                // Format as YYYY-MM-DD HH:MM:SS
                let date = dt.date();
                let time = dt.time();
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    date.year(),
                    date.month() as u8,
                    date.day(),
                    time.hours(),
                    time.minutes(),
                    time.seconds()
                )
            }
            FieldValue::Double(d) => d.to_string(),
            FieldValue::Memo(s) => s.clone(),
        };
        csv_fields.push(field_str);
    }

    Ok(csv_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        use common::models::config::*;

        Config {
            scheduler: SchedulerConfig {
                crontab: "*/5 * * * *".to_string(),
            },
            src: SourceConfig {
                source_dir: std::path::PathBuf::from("/tmp"),
                include_patterns: None,
                exclude_patterns: None,
            },
            credential: CredentialConfig {
                account: "testaccount".to_string(),
                username: "test".to_string(),
                password: "test".to_string(),
                device: None,
            },
            api: ApiConfig {
                base_url: "https://api.test.com".to_string(),
                https_only: true,
            },
            encoding: EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    #[test]
    fn test_get_encoding_enum() {
        let config = create_test_config();

        // Test known encodings
        assert_eq!(
            get_encoding_enum(Some(&Encoding::CP866), &config.encoding.dbf_encoding),
            Encoding::CP866
        );
        assert_eq!(
            get_encoding_enum(Some(&Encoding::Windows1251), &config.encoding.dbf_encoding),
            Encoding::Windows1251
        );
        assert_eq!(
            get_encoding_enum(Some(&Encoding::UTF8), &config.encoding.dbf_encoding),
            Encoding::UTF8
        );

        // Test None falls back to config
        let encoding = get_encoding_enum(None, "CP866");
        assert_eq!(encoding, Encoding::CP866);
    }

    #[test]
    fn test_convert_nonexistent_file() {
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let dbf_path = temp_dir.path().join("nonexistent.dbf");
        let dbf_file = DbfFile::new(dbf_path, temp_dir.path());

        let result = convert_dbf_to_csv(&dbf_file, &config);
        assert!(result.is_err());
    }

    // Note: Full integration tests with actual DBF files would require sample DBF files
    // These would be better placed in integration tests with fixtures
}
