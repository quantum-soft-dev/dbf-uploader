/// Version information for the application
pub struct Version;

impl Version {
    /// Get the version string
    ///
    /// On main branch: returns just the version (e.g., "1.0.0")
    /// On other branches: returns version with git hash (e.g., "1.0.0-dev+abc1234")
    pub fn get() -> String {
        let base_version = env!("CARGO_PKG_VERSION");
        let git_sha = option_env!("VERGEN_GIT_SHA");
        let git_branch = option_env!("VERGEN_GIT_BRANCH");

        match (git_sha, git_branch) {
            (Some(sha), Some(branch)) => {
                // Take first 7 characters of git SHA
                let short_sha = &sha[..7.min(sha.len())];

                // On main branch, return just the version
                if branch == "main" || branch == "refs/heads/main" {
                    base_version.to_string()
                } else {
                    // On other branches, append -dev+{git_hash}
                    format!("{}-dev+{}", base_version, short_sha)
                }
            }
            _ => {
                // Fallback if git info not available
                base_version.to_string()
            }
        }
    }

    /// Get detailed version information including build timestamp and rustc version
    pub fn detailed() -> String {
        let version = Self::get();
        let build_timestamp = option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown");
        let rustc_version = option_env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown");

        format!(
            "{} (built {} with rustc {})",
            version, build_timestamp, rustc_version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_get() {
        let version = Version::get();
        assert!(!version.is_empty());
        // Should start with the cargo version
        assert!(version.starts_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_version_detailed() {
        let detailed = Version::detailed();
        assert!(!detailed.is_empty());
        assert!(detailed.contains("built"));
        assert!(detailed.contains("rustc"));
    }
}
