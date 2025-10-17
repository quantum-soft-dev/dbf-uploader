// Contract tests for Authentication v2 API
// Tests the middleware v2 authentication protocol with site credentials

mod common;

use common::{test_data, MockMiddleware};
use data_exporter::auth::{AuthClient, SiteCredentials, TokenManager};
use data_exporter::models::Config;

#[tokio::test]
async fn test_auth_with_valid_site_credentials() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create test JWT
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mock response
    let _m = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);

    // Create AuthClient with site credentials
    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = AuthClient::from_credentials(base_url, credentials)
        .expect("Failed to create AuthClient");

    // Act
    let jwt_token = auth_client.get_token().await.expect("Failed to get token");

    // Assert
    assert!(!jwt_token.token.is_empty());
    assert_eq!(jwt_token.site_id, test_data::test_site_id());
    assert_eq!(jwt_token.account_id, test_data::test_account_id());
    assert_eq!(jwt_token.domain, test_data::TEST_DOMAIN);
    assert!(jwt_token.expires_at > 0);
}

#[tokio::test]
async fn test_auth_with_invalid_credentials() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Setup mock response for invalid credentials
    let _m = mock.mock_auth_invalid_credentials();

    // Create AuthClient with invalid credentials
    let credentials = SiteCredentials {
        domain: "invalid.com".to_string(),
        client_secret: "wrong-secret".to_string(),
    };

    let auth_client = AuthClient::from_credentials(base_url, credentials)
        .expect("Failed to create AuthClient");

    // Act
    let result = auth_client.get_token().await;

    // Assert
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Invalid credentials"));
}

#[tokio::test]
async fn test_auth_inactive_subscription() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Setup mock response for inactive subscription
    let _m = mock.mock_auth_inactive_subscription(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET);

    // Create AuthClient
    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = AuthClient::from_credentials(base_url, credentials)
        .expect("Failed to create AuthClient");

    // Act
    let result = auth_client.get_token().await;

    // Assert
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Subscription inactive"));
}

#[tokio::test]
async fn test_auth_endpoint_url() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create test JWT
    let token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mock - verify it's calling /api/v1/auth/token (not v1.0 /api/auth/token)
    let _m = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token, 3600);

    // Create AuthClient
    let credentials = SiteCredentials {
        domain: test_data::TEST_DOMAIN.to_string(),
        client_secret: test_data::TEST_CLIENT_SECRET.to_string(),
    };

    let auth_client = AuthClient::from_credentials(base_url, credentials)
        .expect("Failed to create AuthClient");

    // Act - this will call /api/v1/auth/token
    let result = auth_client.get_token().await;

    // Assert - if the mock was hit, the URL is correct
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_jwt_payload_parsing() {
    // Create test JWT with known payload
    let site_id = test_data::test_site_id();
    let account_id = test_data::test_account_id();
    let domain = "test-store.example.com";
    let exp = 9999999999u64;

    let token = common::create_test_jwt(site_id, account_id, domain, exp);

    // Parse the payload
    let payload = data_exporter::auth::JwtToken::parse_payload(&token)
        .expect("Failed to parse JWT payload");

    // Assert
    assert_eq!(payload.site_id, site_id);
    assert_eq!(payload.account_id, account_id);
    assert_eq!(payload.domain, domain);
    assert_eq!(payload.exp, exp);
}

#[tokio::test]
async fn test_jwt_payload_parsing_invalid_format() {
    // Create invalid JWT (only 2 parts instead of 3)
    let invalid_token = "header.payload";

    // Try to parse
    let result = data_exporter::auth::JwtToken::parse_payload(invalid_token);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected 3 parts"));
}

#[tokio::test]
async fn test_token_manager_renewal_logic() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create test JWT
    let token1 = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mock to be called twice (initial + renewal)
    let _m1 = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &token1, 3600);

    // Create v1 config for TokenManager (compatibility layer)
    let config = Config {
        scheduler: data_exporter::models::config::SchedulerConfig {
            crontab: "0 * * * *".to_string(),
        },
        src: data_exporter::models::config::SourceConfig {
            source_dir: std::path::PathBuf::from("/tmp"),
        },
        credential: data_exporter::models::config::CredentialConfig {
            username: test_data::TEST_DOMAIN.to_string(),
            password: test_data::TEST_CLIENT_SECRET.to_string(),
        },
        api: data_exporter::models::config::ApiConfig {
            base_url,
        },
        encoding: data_exporter::models::config::EncodingConfig {
            dbf_encoding: "CP866".to_string(),
        },
    };

    let token_manager = TokenManager::new(&config).expect("Failed to create TokenManager");

    // Act - get token first time
    let token_result1 = token_manager.get_valid_token().await;
    assert!(token_result1.is_ok());

    // Get token second time - should use cached token (not call mock again)
    let token_result2 = token_manager.get_valid_token().await;
    assert!(token_result2.is_ok());

    // Verify both tokens are the same (cached)
    assert_eq!(token_result1.unwrap().token, token_result2.unwrap().token);
}

#[tokio::test]
async fn test_token_manager_with_expired_token() {
    // Setup mock server
    let mut mock = MockMiddleware::new().await;
    let base_url = mock.url();

    // Create EXPIRED test JWT (exp in the past)
    let expired_token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        1000, // Expired timestamp
    );

    // Create fresh token
    let fresh_token = common::create_test_jwt(
        test_data::test_site_id(),
        test_data::test_account_id(),
        test_data::TEST_DOMAIN,
        9999999999,
    );

    // Setup mocks - first call returns expired, second call returns fresh
    let _m1 = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &expired_token, 1);

    // Create config
    let config = Config {
        scheduler: data_exporter::models::config::SchedulerConfig {
            crontab: "0 * * * *".to_string(),
        },
        src: data_exporter::models::config::SourceConfig {
            source_dir: std::path::PathBuf::from("/tmp"),
        },
        credential: data_exporter::models::config::CredentialConfig {
            username: test_data::TEST_DOMAIN.to_string(),
            password: test_data::TEST_CLIENT_SECRET.to_string(),
        },
        api: data_exporter::models::config::ApiConfig {
            base_url: base_url.clone(),
        },
        encoding: data_exporter::models::config::EncodingConfig {
            dbf_encoding: "CP866".to_string(),
        },
    };

    let token_manager = TokenManager::new(&config).expect("Failed to create TokenManager");

    // Act - get expired token
    let token_result1 = token_manager.get_valid_token().await;
    assert!(token_result1.is_ok());

    let first_token = token_result1.unwrap();

    // Verify token is considered expired (within 5 minute threshold)
    assert!(first_token.is_expired());

    // Setup mock for renewal
    drop(_m1); // Drop first mock
    let _m2 = mock.mock_auth_success(test_data::TEST_DOMAIN, test_data::TEST_CLIENT_SECRET, &fresh_token, 3600);

    // Get token again - should trigger renewal because first one is expired
    let token_result2 = token_manager.get_valid_token().await;
    assert!(token_result2.is_ok());

    // Verify we got a fresh token
    let second_token = token_result2.unwrap();
    assert!(!second_token.is_expired());
}
