use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Site credentials for authentication with the middleware
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteCredentials {
    /// Site domain (e.g., "store-01.example.com")
    pub domain: String,
    /// Client secret UUID from middleware admin API
    pub client_secret: String,
}

impl SiteCredentials {
    /// Create new SiteCredentials with validation
    pub fn new(domain: String, client_secret: String) -> Result<Self, String> {
        let credentials = Self {
            domain,
            client_secret,
        };
        credentials.validate()?;
        Ok(credentials)
    }

    /// Validate both domain and client_secret formats
    pub fn validate(&self) -> Result<(), String> {
        self.validate_domain()?;
        self.validate_client_secret()?;
        Ok(())
    }

    /// Validate domain is DNS-compliant
    /// Rules:
    /// - Length: 1-253 characters
    /// - Labels separated by dots
    /// - Each label: 1-63 characters, alphanumeric and hyphens
    /// - Labels cannot start or end with hyphen
    /// - At least one dot required (subdomain.domain.tld)
    pub fn validate_domain(&self) -> Result<(), String> {
        let domain = &self.domain;

        // Check overall length
        if domain.is_empty() {
            return Err("Domain cannot be empty".to_string());
        }
        if domain.len() > 253 {
            return Err(format!(
                "Domain too long: {} characters (max 253)",
                domain.len()
            ));
        }

        // Must contain at least one dot
        if !domain.contains('.') {
            return Err(
                "Domain must contain at least one dot (e.g., subdomain.example.com)".to_string(),
            );
        }

        // Validate each label
        let labels: Vec<&str> = domain.split('.').collect();
        for (i, label) in labels.iter().enumerate() {
            if label.is_empty() {
                return Err(format!("Domain label {} is empty", i + 1));
            }
            if label.len() > 63 {
                return Err(format!(
                    "Domain label '{}' too long: {} characters (max 63)",
                    label,
                    label.len()
                ));
            }

            // Check valid characters and structure
            for (j, ch) in label.chars().enumerate() {
                let is_first = j == 0;
                let is_last = j == label.len() - 1;

                if ch.is_ascii_alphanumeric() {
                    continue;
                }
                if ch == '-' {
                    if is_first {
                        return Err(format!("Domain label '{}' cannot start with hyphen", label));
                    }
                    if is_last {
                        return Err(format!("Domain label '{}' cannot end with hyphen", label));
                    }
                    continue;
                }
                return Err(format!(
                    "Domain label '{}' contains invalid character: '{}'",
                    label, ch
                ));
            }
        }

        Ok(())
    }

    /// Validate client_secret is a valid UUID
    pub fn validate_client_secret(&self) -> Result<(), String> {
        Uuid::parse_str(&self.client_secret)
            .map_err(|e| format!("Invalid client_secret UUID format: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_credentials() {
        let creds = SiteCredentials::new(
            "store-01.example.com".to_string(),
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
        );
        assert!(creds.is_ok());
    }

    #[test]
    fn test_valid_domain_formats() {
        let valid_domains = vec![
            "a.b",
            "store.example.com",
            "store-01.example.com",
            "my-store-123.sub.example.co.uk",
            "123.456.789.com",
        ];

        for domain in valid_domains {
            let result = SiteCredentials {
                domain: domain.to_string(),
                client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            }
            .validate_domain();
            assert!(
                result.is_ok(),
                "Domain '{}' should be valid: {:?}",
                domain,
                result
            );
        }
    }

    #[test]
    fn test_invalid_domain_empty() {
        let creds = SiteCredentials {
            domain: "".to_string(),
            client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let result = creds.validate_domain();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_invalid_domain_no_dot() {
        let creds = SiteCredentials {
            domain: "localhost".to_string(),
            client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let result = creds.validate_domain();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one dot"));
    }

    #[test]
    fn test_invalid_domain_too_long() {
        let long_label = "a".repeat(64);
        let creds = SiteCredentials {
            domain: format!("{}.example.com", long_label),
            client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let result = creds.validate_domain();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_invalid_domain_starts_with_hyphen() {
        let creds = SiteCredentials {
            domain: "-store.example.com".to_string(),
            client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let result = creds.validate_domain();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot start with hyphen"));
    }

    #[test]
    fn test_invalid_domain_ends_with_hyphen() {
        let creds = SiteCredentials {
            domain: "store-.example.com".to_string(),
            client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let result = creds.validate_domain();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot end with hyphen"));
    }

    #[test]
    fn test_invalid_domain_special_chars() {
        let invalid_domains = vec![
            "store_01.example.com",
            "store@example.com",
            "store#01.example.com",
        ];

        for domain in invalid_domains {
            let creds = SiteCredentials {
                domain: domain.to_string(),
                client_secret: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            };
            let result = creds.validate_domain();
            assert!(result.is_err(), "Domain '{}' should be invalid", domain);
            assert!(result.unwrap_err().contains("invalid character"));
        }
    }

    #[test]
    fn test_valid_client_secret_formats() {
        let valid_uuids = vec![
            "550e8400-e29b-41d4-a716-446655440000",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "00000000-0000-0000-0000-000000000000",
            "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
        ];

        for uuid in valid_uuids {
            let creds = SiteCredentials {
                domain: "store.example.com".to_string(),
                client_secret: uuid.to_string(),
            };
            let result = creds.validate_client_secret();
            assert!(
                result.is_ok(),
                "UUID '{}' should be valid: {:?}",
                uuid,
                result
            );
        }
    }

    #[test]
    fn test_invalid_client_secret_formats() {
        let invalid_uuids = vec![
            "not-a-uuid",
            "550e8400-e29b-41d4-a716",
            "550e8400-e29b-41d4-a716-446655440000-extra",
            "",
            "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        ];

        for uuid in invalid_uuids {
            let creds = SiteCredentials {
                domain: "store.example.com".to_string(),
                client_secret: uuid.to_string(),
            };
            let result = creds.validate_client_secret();
            assert!(result.is_err(), "UUID '{}' should be invalid", uuid);
            assert!(result.unwrap_err().contains("Invalid client_secret UUID"));
        }
    }

    #[test]
    fn test_new_with_valid_inputs() {
        let result = SiteCredentials::new(
            "store.example.com".to_string(),
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_with_invalid_domain() {
        let result = SiteCredentials::new(
            "localhost".to_string(),
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_new_with_invalid_uuid() {
        let result =
            SiteCredentials::new("store.example.com".to_string(), "not-a-uuid".to_string());
        assert!(result.is_err());
    }
}
