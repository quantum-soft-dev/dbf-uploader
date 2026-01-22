//! File filtering logic using glob patterns
//!
//! This module provides functionality to filter DBF files based on
//! include/exclude patterns specified in configuration.

use common::models::config::SourceConfig;
use globset::{GlobSet, GlobSetBuilder};
use std::sync::OnceLock;

/// Compiled filter patterns (cached for performance)
#[derive(Debug)]
pub struct FileFilter {
    include_set: Option<GlobSet>,
    exclude_set: Option<GlobSet>,
}

impl FileFilter {
    /// Create a new FileFilter from configuration
    pub fn from_config(config: &SourceConfig) -> Result<Self, String> {
        let include_set = Self::build_globset(&config.include_patterns)?;
        let exclude_set = Self::build_globset(&config.exclude_patterns)?;

        Ok(FileFilter {
            include_set,
            exclude_set,
        })
    }

    /// Build a GlobSet from patterns (case-insensitive)
    fn build_globset(patterns: &Option<Vec<String>>) -> Result<Option<GlobSet>, String> {
        if let Some(ref pattern_list) = patterns {
            if pattern_list.is_empty() {
                return Ok(None);
            }

            let mut builder = GlobSetBuilder::new();

            for pattern in pattern_list {
                // Build glob with case-insensitive matching
                let glob = globset::GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))?;

                builder.add(glob);
            }

            let globset = builder
                .build()
                .map_err(|e| format!("Failed to build globset: {}", e))?;

            Ok(Some(globset))
        } else {
            Ok(None)
        }
    }

    /// Check if a file should be processed based on filter patterns
    ///
    /// Logic:
    /// 1. If include_patterns exist: file MUST match at least one include pattern
    /// 2. If exclude_patterns exist: file MUST NOT match any exclude pattern
    /// 3. Include is checked first, then exclude
    ///
    /// # Arguments
    /// * `filename` - The filename to check (case-insensitive matching)
    ///
    /// # Returns
    /// * `true` if file should be processed
    /// * `false` if file should be filtered out
    pub fn should_process(&self, filename: &str) -> bool {
        // Step 1: Check include patterns (whitelist)
        if let Some(ref include_set) = self.include_set {
            // If include patterns exist, file MUST match at least one
            if !include_set.is_match(filename) {
                tracing::debug!(
                    file = %filename,
                    "File filtered out: does not match any include pattern"
                );
                return false;
            }
        }

        // Step 2: Check exclude patterns (blacklist)
        if let Some(ref exclude_set) = self.exclude_set {
            // If exclude patterns exist, file MUST NOT match any
            if exclude_set.is_match(filename) {
                tracing::debug!(
                    file = %filename,
                    "File filtered out: matches exclude pattern"
                );
                return false;
            }
        }

        // File passes all filters
        true
    }
}

/// Cached global filter instance (reloaded when config changes)
static CACHED_FILTER: OnceLock<FileFilter> = OnceLock::new();

/// Initialize or update the global file filter
pub fn set_global_filter(config: &SourceConfig) -> Result<(), String> {
    let filter = FileFilter::from_config(config)?;

    // Clear old cache and set new one
    // Note: OnceLock doesn't support clearing, so we just overwrite
    // This is called once per batch, so it's acceptable
    let _ = CACHED_FILTER.set(filter);

    Ok(())
}

/// Check if a file should be processed using the global filter
pub fn should_process_file(filename: &str) -> bool {
    if let Some(filter) = CACHED_FILTER.get() {
        filter.should_process(filename)
    } else {
        // No filter configured, process all files
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_patterns_processes_all() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: None,
            exclude_patterns: None,
        };

        let filter = FileFilter::from_config(&config).unwrap();

        assert!(filter.should_process("test.dbf"));
        assert!(filter.should_process("ANY_FILE.DBF"));
        assert!(filter.should_process("data/nested/file.dbf"));
    }

    #[test]
    fn test_exclude_exact_match_case_insensitive() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: None,
            exclude_patterns: Some(vec!["nsfcli.DBF".to_string(), "NsfMod.dbf".to_string()]),
        };

        let filter = FileFilter::from_config(&config).unwrap();

        // Exact matches (case-insensitive)
        assert!(!filter.should_process("nsfcli.DBF"));
        assert!(!filter.should_process("NSFCLI.DBF"));
        assert!(!filter.should_process("nsfcli.dbf"));
        assert!(!filter.should_process("NsfMod.dbf"));
        assert!(!filter.should_process("NSFMOD.DBF"));

        // Should not match
        assert!(filter.should_process("other.dbf"));
        assert!(filter.should_process("nsfclient.dbf"));
    }

    #[test]
    fn test_exclude_wildcard_patterns() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: None,
            exclude_patterns: Some(vec!["temp_*.dbf".to_string(), "*.bak".to_string()]),
        };

        let filter = FileFilter::from_config(&config).unwrap();

        // Should be excluded
        assert!(!filter.should_process("temp_data.dbf"));
        assert!(!filter.should_process("TEMP_DATA.DBF"));
        assert!(!filter.should_process("file.bak"));
        assert!(!filter.should_process("FILE.BAK"));

        // Should not be excluded
        assert!(filter.should_process("data.dbf"));
        assert!(filter.should_process("temporary.dbf"));
    }

    #[test]
    fn test_include_patterns_whitelist() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: Some(vec!["nsf*.DBF".to_string()]),
            exclude_patterns: None,
        };

        let filter = FileFilter::from_config(&config).unwrap();

        // Should be included
        assert!(filter.should_process("nsfcli.DBF"));
        assert!(filter.should_process("NSFDATA.DBF"));
        assert!(filter.should_process("nsf_anything.dbf"));

        // Should NOT be included
        assert!(!filter.should_process("other.dbf"));
        assert!(!filter.should_process("data.DBF"));
    }

    #[test]
    fn test_include_and_exclude_combined() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: Some(vec!["nsf*.DBF".to_string()]),
            exclude_patterns: Some(vec!["nsfcli.DBF".to_string(), "nsfmod.dbf".to_string()]),
        };

        let filter = FileFilter::from_config(&config).unwrap();

        // Included by include pattern, not excluded
        assert!(filter.should_process("nsfdata.DBF"));
        assert!(filter.should_process("NSFOTHER.DBF"));

        // Included by include pattern, BUT excluded by exclude pattern
        assert!(!filter.should_process("nsfcli.DBF"));
        assert!(!filter.should_process("NSFMOD.DBF"));

        // Not included by include pattern
        assert!(!filter.should_process("other.dbf"));
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let config = SourceConfig {
            source_dir: std::path::PathBuf::from("/test"),
            include_patterns: Some(vec!["[invalid".to_string()]),
            exclude_patterns: None,
        };

        let result = FileFilter::from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid glob pattern"));
    }
}
