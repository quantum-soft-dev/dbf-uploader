// Test fixture generation script
// Run with: cargo run --example create_fixtures
// Or use as a module in tests

use dbase::{FieldValue, Record, TableWriterBuilder, FieldName, FieldInfo};
use dbase::encoding::LossyCodePage;
use dbase::yore::code_pages::{CP1251, CP1255, CP866};
use std::path::Path;

/// Create a test DBF file with CP866 Cyrillic encoding
/// Contains Russian text that should decode properly
pub fn create_cp866_fixture(path: &Path) -> std::io::Result<()> {
    let fields = vec![
        FieldInfo::new(
            FieldName::try_from("NAME").unwrap(),
            dbase::FieldType::Character(50),
        ),
        FieldInfo::new(
            FieldName::try_from("CITY").unwrap(),
            dbase::FieldType::Character(50),
        ),
        FieldInfo::new(
            FieldName::try_from("AGE").unwrap(),
            dbase::FieldType::Numeric { length: 10, num_decimal_places: 0 },
        ),
    ];

    let writer = TableWriterBuilder::new()
        .set_encoding(LossyCodePage(CP866))
        .add_field(fields[0].clone())
        .add_field(fields[1].clone())
        .add_field(fields[2].clone());

    let mut file_writer = writer.build_with_file_dest(path)?;

    // CP866 Russian text samples (will be encoded from UTF-8 to CP866)
    let records = vec![
        ("Иван", "Москва", 25),
        ("Мария", "Санкт-Петербург", 32),
        ("Алексей", "Новосибирск", 28),
        ("Елена", "Екатеринбург", 45),
        ("Дмитрий", "Казань", 19),
    ];

    for (name, city, age) in records {
        let mut record = Record::default();
        record.insert("NAME".to_string(), FieldValue::Character(Some(name.to_string())));
        record.insert("CITY".to_string(), FieldValue::Character(Some(city.to_string())));
        record.insert("AGE".to_string(), FieldValue::Numeric(Some(age as f64)));
        file_writer.write_record(&record)?;
    }

    file_writer.close()
}

/// Create a test DBF file with Windows-1251 Cyrillic encoding
/// Contains Russian text similar to CP866 but with 1251 encoding
pub fn create_cp1251_fixture(path: &Path) -> std::io::Result<()> {
    let fields = vec![
        FieldInfo::new(
            FieldName::try_from("PRODUCT").unwrap(),
            dbase::FieldType::Character(60),
        ),
        FieldInfo::new(
            FieldName::try_from("DESC").unwrap(),
            dbase::FieldType::Character(100),
        ),
        FieldInfo::new(
            FieldName::try_from("PRICE").unwrap(),
            dbase::FieldType::Numeric { length: 12, num_decimal_places: 2 },
        ),
    ];

    let writer = TableWriterBuilder::new()
        .set_encoding(LossyCodePage(CP1251))
        .add_field(fields[0].clone())
        .add_field(fields[1].clone())
        .add_field(fields[2].clone());

    let mut file_writer = writer.build_with_file_dest(path)?;

    // CP1251 Russian product data
    let records = vec![
        ("Компьютер", "Персональный компьютер для офиса", 45000.00),
        ("Монитор", "ЖК монитор 24 дюйма", 12500.50),
        ("Клавиатура", "Механическая клавиатура", 3200.00),
        ("Мышь", "Беспроводная мышь", 1500.99),
    ];

    for (product, desc, price) in records {
        let mut record = Record::default();
        record.insert("PRODUCT".to_string(), FieldValue::Character(Some(product.to_string())));
        record.insert("DESC".to_string(), FieldValue::Character(Some(desc.to_string())));
        record.insert("PRICE".to_string(), FieldValue::Numeric(Some(price)));
        file_writer.write_record(&record)?;
    }

    file_writer.close()
}

/// Create a test DBF file with Windows-1255 Hebrew encoding
/// Contains Hebrew text for testing RTL language support
pub fn create_cp1255_fixture(path: &Path) -> std::io::Result<()> {
    let fields = vec![
        FieldInfo::new(
            FieldName::try_from("SHEMID").unwrap(), // Name ID in Hebrew style
            dbase::FieldType::Character(50),
        ),
        FieldInfo::new(
            FieldName::try_from("VALUE").unwrap(),
            dbase::FieldType::Numeric { length: 10, num_decimal_places: 2 },
        ),
        FieldInfo::new(
            FieldName::try_from("ACTIVE").unwrap(),
            dbase::FieldType::Logical,
        ),
    ];

    let writer = TableWriterBuilder::new()
        .set_encoding(LossyCodePage(CP1255))
        .add_field(fields[0].clone())
        .add_field(fields[1].clone())
        .add_field(fields[2].clone());

    let mut file_writer = writer.build_with_file_dest(path)?;

    // CP1255 Hebrew text samples
    let records = vec![
        ("שלום", 100.50, true),
        ("עולם", 200.00, true),
        ("בדיקה", 50.25, false),
    ];

    for (name, value, active) in records {
        let mut record = Record::default();
        record.insert("SHEMID".to_string(), FieldValue::Character(Some(name.to_string())));
        record.insert("VALUE".to_string(), FieldValue::Numeric(Some(value)));
        record.insert("ACTIVE".to_string(), FieldValue::Logical(Some(active)));
        file_writer.write_record(&record)?;
    }

    file_writer.close()
}

/// Create a minimal DBF file for testing basic functionality
/// Uses standard ASCII encoding
pub fn create_basic_ascii_fixture(path: &Path) -> std::io::Result<()> {
    let fields = vec![
        FieldInfo::new(
            FieldName::try_from("ID").unwrap(),
            dbase::FieldType::Numeric { length: 10, num_decimal_places: 0 },
        ),
        FieldInfo::new(
            FieldName::try_from("TEXT").unwrap(),
            dbase::FieldType::Character(50),
        ),
    ];

    let writer = TableWriterBuilder::new()
        .add_field(fields[0].clone())
        .add_field(fields[1].clone());

    let mut file_writer = writer.build_with_file_dest(path)?;

    for i in 1..=5 {
        let mut record = Record::default();
        record.insert("ID".to_string(), FieldValue::Numeric(Some(i as f64)));
        record.insert("TEXT".to_string(), FieldValue::Character(Some(format!("Record {}", i))));
        file_writer.write_record(&record)?;
    }

    file_writer.close()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_basic_ascii() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_ascii.dbf");
        create_basic_ascii_fixture(&path).unwrap();
        assert!(path.exists());
    }
}
