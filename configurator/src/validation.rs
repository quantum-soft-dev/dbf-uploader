//! Validation functions for the Configurator
//!
//! These functions are extracted from the UI code to allow unit testing.

use cron::Schedule;
use std::path::Path;
use std::str::FromStr;

/// Validate cron expression, returns Ok(()) or Err(error message)
///
/// Accepts both 5-field (standard cron) and 6-field (with seconds) formats.
pub fn validate_cron(cron_text: &str) -> Result<(), String> {
    let cron_text = cron_text.trim();

    if cron_text.is_empty() {
        return Err("Cron expression is required".to_string());
    }

    // Convert 5-field cron to 6-field for validation (add seconds)
    let cron_6field = if cron_text.split_whitespace().count() == 5 {
        format!("0 {}", cron_text)
    } else {
        cron_text.to_string()
    };

    Schedule::from_str(&cron_6field)
        .map(|_| ())
        .map_err(|e| format!("Invalid cron expression: {}", e))
}

/// Validate glob patterns, returns Ok(()) or Err(error message)
pub fn validate_patterns(patterns: &Option<Vec<String>>, field_name: &str) -> Result<(), String> {
    if let Some(ref pattern_list) = patterns {
        for pattern in pattern_list {
            if let Err(e) = globset::GlobBuilder::new(pattern).build() {
                return Err(format!(
                    "Invalid {} pattern '{}': {}",
                    field_name, pattern, e
                ));
            }
        }
    }
    Ok(())
}

/// Validate HTTPS URL requirement
///
/// Returns Ok(()) if:
/// - https_only is false (any URL allowed)
/// - https_only is true and URL starts with https://
///
/// Returns Err with message if https_only is true but URL doesn't use HTTPS.
pub fn validate_https_url(url: &str, https_only: bool) -> Result<(), String> {
    if !https_only {
        return Ok(());
    }

    let url = url.trim().to_lowercase();
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err("Server URL must use HTTPS when https_only is enabled".to_string())
    }
}

/// Validate source directory exists
///
/// Returns Ok(true) if directory exists, Ok(false) if it doesn't exist.
/// This allows the caller to decide whether to show a warning or error.
pub fn validate_source_directory(path: &Path) -> bool {
    path.exists()
}

/// Parse comma-separated pattern string into Option<Vec<String>>
///
/// Returns None if input is empty or contains only whitespace.
/// Filters out empty patterns from the result.
pub fn parse_patterns(input: &str) -> Option<Vec<String>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        let patterns: Vec<String> = trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // === Cron Validation Tests ===

    #[test]
    fn test_cron_valid_5_field() {
        assert!(validate_cron("*/5 * * * *").is_ok());
        assert!(validate_cron("0 8,12,16,18 * * *").is_ok());
        assert!(validate_cron("30 9 * * 1-5").is_ok());
    }

    #[test]
    fn test_cron_valid_6_field() {
        assert!(validate_cron("0 */5 * * * *").is_ok());
        assert!(validate_cron("0 0 8,12,16,18 * * *").is_ok());
    }

    #[test]
    fn test_cron_empty_returns_error() {
        let result = validate_cron("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("required"));
    }

    #[test]
    fn test_cron_whitespace_only_returns_error() {
        let result = validate_cron("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_invalid_expression() {
        let result = validate_cron("invalid cron");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cron"));
    }

    #[test]
    fn test_cron_trims_whitespace() {
        assert!(validate_cron("  */5 * * * *  ").is_ok());
    }

    // === HTTPS URL Validation Tests ===

    #[test]
    fn test_https_url_valid() {
        assert!(validate_https_url("https://example.com", true).is_ok());
        assert!(validate_https_url("HTTPS://EXAMPLE.COM", true).is_ok());
        assert!(validate_https_url("  https://example.com  ", true).is_ok());
    }

    #[test]
    fn test_https_url_http_rejected_when_https_only() {
        let result = validate_https_url("http://example.com", true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTPS"));
    }

    #[test]
    fn test_https_url_http_allowed_when_not_https_only() {
        assert!(validate_https_url("http://example.com", false).is_ok());
    }

    #[test]
    fn test_https_url_no_protocol_rejected() {
        let result = validate_https_url("example.com", true);
        assert!(result.is_err());
    }

    // === Pattern Validation Tests ===

    #[test]
    fn test_patterns_valid() {
        let patterns = Some(vec!["*.dbf".to_string(), "data_*.dbf".to_string()]);
        assert!(validate_patterns(&patterns, "include").is_ok());
    }

    #[test]
    fn test_patterns_none_is_ok() {
        assert!(validate_patterns(&None, "include").is_ok());
    }

    #[test]
    fn test_patterns_empty_vec_is_ok() {
        let patterns = Some(vec![]);
        assert!(validate_patterns(&patterns, "include").is_ok());
    }

    #[test]
    fn test_patterns_invalid_glob() {
        let patterns = Some(vec!["[invalid".to_string()]);
        let result = validate_patterns(&patterns, "include");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid include pattern"));
    }

    // === Parse Patterns Tests ===

    #[test]
    fn test_parse_patterns_valid() {
        let result = parse_patterns("*.dbf, data_*.dbf");
        assert_eq!(
            result,
            Some(vec!["*.dbf".to_string(), "data_*.dbf".to_string()])
        );
    }

    #[test]
    fn test_parse_patterns_empty_returns_none() {
        assert_eq!(parse_patterns(""), None);
        assert_eq!(parse_patterns("   "), None);
    }

    #[test]
    fn test_parse_patterns_filters_empty() {
        let result = parse_patterns("*.dbf, , ,data_*.dbf");
        assert_eq!(
            result,
            Some(vec!["*.dbf".to_string(), "data_*.dbf".to_string()])
        );
    }

    #[test]
    fn test_parse_patterns_only_commas_returns_none() {
        assert_eq!(parse_patterns(", , ,"), None);
    }

    #[test]
    fn test_parse_patterns_trims_whitespace() {
        let result = parse_patterns("  *.dbf  ,  data_*.dbf  ");
        assert_eq!(
            result,
            Some(vec!["*.dbf".to_string(), "data_*.dbf".to_string()])
        );
    }

    // === Source Directory Validation Tests ===

    #[test]
    fn test_source_directory_exists() {
        // Current directory should always exist
        let path = PathBuf::from(".");
        assert!(validate_source_directory(&path));
    }

    #[test]
    fn test_source_directory_not_exists() {
        let path = PathBuf::from("/nonexistent/directory/12345");
        assert!(!validate_source_directory(&path));
    }
}
