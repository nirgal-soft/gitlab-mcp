use gitlab_mcp::{config::Config, Server};

#[tokio::test]
async fn test_server_creation() {
    // Create test config
    let config = Config {
        server: gitlab_mcp::config::ServerConfig {
            name: "test-server".to_string(),
            transport: gitlab_mcp::config::TransportType::Stdio,
        },
        telemetry: gitlab_mcp::config::TelemetryConfig {
            level: "error".to_string(),
            format: gitlab_mcp::config::LogFormat::Pretty,
            file: None,
        },
    };

    // Test server creation - this should work without any complex setup
    let server = Server::new(config).await;
    assert!(server.is_ok(), "Server creation should succeed");

    let _server = server.unwrap();
    // Just verify we can create the server - that's the main integration point
    // The ServerHandler methods would typically be called by the rmcp framework, not directly in tests
}

#[tokio::test]
async fn test_config_validation() {
    // Test with a different configuration to ensure flexibility
    let config = Config {
        server: gitlab_mcp::config::ServerConfig {
            name: "test-config-server".to_string(),
            transport: gitlab_mcp::config::TransportType::Stdio,
        },
        telemetry: gitlab_mcp::config::TelemetryConfig {
            level: "debug".to_string(),
            format: gitlab_mcp::config::LogFormat::Json,
            file: Some("/tmp/test.log".to_string()),
        },
    };

    let server = Server::new(config).await;
    assert!(
        server.is_ok(),
        "Server should handle different config options"
    );
}
