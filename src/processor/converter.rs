// DBF to CSV converter
use crate::error::{ProcessingError, Result};
use crate::models::{Config, DbfFile, Encoding};
use csv::Writer;
use dbase::{FieldValue, Record};
use encoding_rs::{Encoding as EncodingRs, WINDOWS_1251};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tracing::{debug, warn};

/// Convert a DBF file to CSV with UTF-8 encoding
/// Returns the path to the created CSV file
pub fn convert_dbf_to_csv(dbf_file: &DbfFile, config: &Config) -> Result<PathBuf> {
    debug!("Converting DBF to CSV: {}", dbf_file.path.display());

    // Open DBF file
    let file = File::open(&dbf_file.path).map_err(ProcessingError::FileReadError)?;

    let mut reader = dbase::Reader::new(BufReader::new(file))
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

    // Determine encoding to use for text fields
    let encoding = get_encoding_for_dbf(dbf_file.encoding.as_ref(), &config.encoding.dbf_encoding);

    // Process each record
    let mut record_count = 0;
    for result in reader.iter_records() {
        match result {
            Ok(record) => {
                let csv_record = convert_record_to_csv(&record, &field_names, encoding)?;
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

/// Get the encoding to use for DBF text fields
fn get_encoding_for_dbf(
    dbf_encoding: Option<&Encoding>,
    config_encoding: &str,
) -> &'static EncodingRs {
    match dbf_encoding {
        Some(Encoding::CP866) => encoding_rs::IBM866,
        Some(Encoding::Windows1251) => WINDOWS_1251,
        Some(Encoding::UTF8) => encoding_rs::UTF_8,
        None => {
            // Use config fallback encoding
            match config_encoding.to_uppercase().as_str() {
                "CP866" | "IBM866" => encoding_rs::IBM866,
                "WINDOWS-1251" | "WINDOWS1251" | "CP1251" => WINDOWS_1251,
                "UTF-8" | "UTF8" => encoding_rs::UTF_8,
                _ => {
                    warn!(
                        "Unknown encoding '{}', defaulting to Windows-1251",
                        config_encoding
                    );
                    WINDOWS_1251
                }
            }
        }
    }
}

/// Convert a DBF record to CSV string fields
/// Takes field names to ensure proper ordering (Record is a HashMap)
fn convert_record_to_csv(
    record: &Record,
    field_names: &[String],
    encoding: &'static EncodingRs,
) -> Result<Vec<String>> {
    let mut csv_fields = Vec::new();

    // Iterate through field names in order to maintain column order
    for field_name in field_names {
        let field_value = record.get(field_name).ok_or_else(|| {
            ProcessingError::ConversionError(format!("Missing field '{}' in record", field_name))
        })?;

        let field_str = match field_value {
            FieldValue::Character(Some(s)) => {
                // Convert from DBF encoding to UTF-8
                if encoding == encoding_rs::UTF_8 {
                    s.clone()
                } else {
                    let bytes = s.as_bytes();
                    let (cow, _encoding_used, had_errors) = encoding.decode(bytes);
                    if had_errors {
                        warn!("Encoding errors detected in text field, some characters may be incorrect");
                    }
                    cow.to_string()
                }
            }
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
            FieldValue::Date(Some(d)) => format!("{:?}", d), // Use Debug format for date
            FieldValue::Date(None) => String::new(),
            FieldValue::Float(Some(f)) => f.to_string(),
            FieldValue::Float(None) => String::new(),
            FieldValue::Integer(i) => i.to_string(),
            FieldValue::Currency(c) => format!("{:.2}", *c / 10000.0),
            FieldValue::DateTime(dt) => format!("{:?}", dt), // Use Debug format for datetime
            FieldValue::Double(d) => d.to_string(),
            FieldValue::Memo(s) => {
                // Convert from DBF encoding to UTF-8 (same as Character)
                if encoding == encoding_rs::UTF_8 {
                    s.clone()
                } else {
                    let bytes = s.as_bytes();
                    let (cow, _encoding_used, had_errors) = encoding.decode(bytes);
                    if had_errors {
                        warn!("Encoding errors detected in memo field");
                    }
                    cow.to_string()
                }
            }
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
        use crate::models::config::*;

        Config {
            scheduler: SchedulerConfig {
                crontab: "*/5 * * * *".to_string(),
            },
            src: SourceConfig {
                source_dir: std::path::PathBuf::from("/tmp"),
            },
            credential: CredentialConfig {
                username: "test".to_string(),
                password: "test".to_string(),
            },
            api: ApiConfig {
                base_url: "https://api.test.com".to_string(),
            },
            encoding: EncodingConfig {
                dbf_encoding: "CP866".to_string(),
            },
        }
    }

    #[test]
    fn test_get_encoding_for_dbf() {
        let config = create_test_config();

        // Test known encodings
        assert_eq!(
            get_encoding_for_dbf(Some(&Encoding::CP866), &config.encoding.dbf_encoding),
            encoding_rs::IBM866
        );
        assert_eq!(
            get_encoding_for_dbf(Some(&Encoding::Windows1251), &config.encoding.dbf_encoding),
            WINDOWS_1251
        );
        assert_eq!(
            get_encoding_for_dbf(Some(&Encoding::UTF8), &config.encoding.dbf_encoding),
            encoding_rs::UTF_8
        );

        // Test None falls back to config
        let encoding = get_encoding_for_dbf(None, "CP866");
        assert_eq!(encoding, encoding_rs::IBM866);
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
