#[cfg(feature = "auth")]
pub mod auth;
pub mod config;
pub mod error;
pub mod gitlab;
pub mod tools;
pub mod state;
pub mod telemetry;

use std::net::SocketAddr;
use rmcp::{
  ServerHandler, ServiceExt,
  tool, tool_handler, tool_router
};
use rmcp::transport::{stdio, streamable_http_server::{StreamableHttpService, StreamableHttpServerConfig}};
use rmcp::model::{*, ErrorData as McpError};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use tower::Service;

use crate::config::Config;
use crate::state::ServerState;
use crate::tools::gitlab::{
  CreateMergeRequestDiscussionRequest,
  CreateMergeRequestNoteRequest,
  GetMergeRequestChangesRequest,
  GetMergeRequestRequest,
  GetMergeRequestVersionsRequest,
  MergeRequestLocator,
  json_result,
  discussion_payload,
  note_payload,
};

#[derive(Clone)]
pub struct Server {
  config: Config,
  #[allow(dead_code)]
  state: ServerState,
  tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Server {
  #[tool(description = "Fetch metadata for a GitLab merge request (title, author, state, approvals, etc.)")]
  pub async fn get_merge_request(
    &self,
    Parameters(req): Parameters<GetMergeRequestRequest>,
  ) -> Result<CallToolResult, McpError>{
    let MergeRequestLocator { project, merge_request_iid } = req.locator;
    let value = self
      .state
      .gitlab
      .get_merge_request(&project, merge_request_iid)
      .await?;

    json_result(value)
  }

  #[tool(description = "Fetch the diff changes for a GitLab merge request (file list and hunks)")]
  pub async fn get_merge_request_changes(
    &self,
    Parameters(req): Parameters<GetMergeRequestChangesRequest>,
  ) -> Result<CallToolResult, McpError>{
    let MergeRequestLocator { project, merge_request_iid } = req.locator;
    let value = self
      .state
      .gitlab
      .get_merge_request_changes(&project, merge_request_iid)
      .await?;

    json_result(value)
  }

  #[tool(description = "Fetch merge request versions (base/head/start commit SHAs for discussions)")]
  pub async fn get_merge_request_versions(
    &self,
    Parameters(req): Parameters<GetMergeRequestVersionsRequest>,
  ) -> Result<CallToolResult, McpError>{
    let MergeRequestLocator { project, merge_request_iid } = req.locator;
    let value = self
      .state
      .gitlab
      .get_merge_request_versions(&project, merge_request_iid)
      .await?;

    json_result(value)
  }

  #[tool(description = "Create a line-level discussion on a GitLab merge request. The position field requires: base_sha, head_sha, start_sha (from get_merge_request_versions), new_path, old_path, and line numbers (new_line for additions, old_line for deletions). Position can be a JSON object or string. The position_type defaults to 'text'.")]
  pub async fn create_merge_request_discussion(
    &self,
    Parameters(req): Parameters<CreateMergeRequestDiscussionRequest>,
  ) -> Result<CallToolResult, McpError>{
    let payload = discussion_payload(&req)?;
    let MergeRequestLocator { project, merge_request_iid } = req.locator;
    let value = self
      .state
      .gitlab
      .create_merge_request_discussion(&project, merge_request_iid, payload)
      .await?;

    json_result(value)
  }

  #[tool(description = "Create a general note on a GitLab merge request (top-level discussion comment)")]
  pub async fn create_merge_request_note(
    &self,
    Parameters(req): Parameters<CreateMergeRequestNoteRequest>,
  ) -> Result<CallToolResult, McpError>{
    let payload = note_payload(&req);
    let MergeRequestLocator { project, merge_request_iid } = req.locator;
    let value = self
      .state
      .gitlab
      .create_merge_request_note(&project, merge_request_iid, payload)
      .await?;

    json_result(value)
  }
}

impl Server {
  pub async fn new(config: Config) -> anyhow::Result<Self> {
    tracing::info!("Initializing MCP Server");
    tracing::info!("Loading server state and tools...");
    
    let state = ServerState::new(&config).await?;
    
    tracing::info!("Server initialization complete");
    Ok(Self { config, state, tool_router: Self::tool_router(), })
  }

  pub async fn run(self) -> anyhow::Result<()> {
    match &self.config.server.transport {
      config::TransportType::Stdio => {
        tracing::info!("MCP Server ready!");
        tracing::info!("Transport: STDIO (Standard Input/Output)");
        
        let transport = stdio();
        let service = self.serve(transport).await?;

        // Set up graceful shutdown
        let shutdown = tokio::spawn(async move {
          tokio::signal::ctrl_c().await.ok();
          tracing::info!("Shutdown signal received");
        });

        tokio::select! {
          result = service.waiting() => {
            tracing::info!("Server stopped: {:?}", result);
          }
          _ = shutdown => {
            tracing::info!("Shutting down gracefully");
          }
        }
      }
      config::TransportType::HttpStreaming { port } => {
        tracing::info!("MCP Server ready!");
        tracing::info!("Transport: HTTP Streaming (using rmcp StreamableHttpService)");
        tracing::info!("Server URL: http://localhost:{}", port);
        
        let addr: SocketAddr = format!("[::]:{}", port).parse().unwrap();
        
        // Create the rmcp StreamableHttpService
        use std::sync::Arc;
        use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
        
        let session_manager = Arc::new(LocalSessionManager::default());
        let config = StreamableHttpServerConfig::default();
        
        let service = StreamableHttpService::new(
          move || Ok(self.clone()),
          session_manager,
          config,
        );
        
        // Create HTTP server using axum
        let app = axum::Router::new()
          .fallback_service(tower::service_fn(move |req| {
            let mut service = service.clone();
            async move { service.call(req).await }
          }));
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let server = axum::serve(listener, app);
        
        // Set up graceful shutdown using the same pattern as STDIO
        let shutdown = tokio::spawn(async move {
          if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for shutdown signal: {}", e);
          }
          tracing::info!("Shutdown signal received");
        });

        tokio::select! {
          result = server => {
            match result {
              Ok(_) => tracing::info!("HTTP server stopped normally"),
              Err(e) => tracing::error!("HTTP server stopped with error: {}", e),
            }
          }
          _ = shutdown => {
            tracing::info!("Shutting down gracefully");
          }
        }
      }
    }

    Ok(())
  }
}

#[tool_handler]
impl ServerHandler for Server {
  fn get_info(&self) -> ServerInfo {
    ServerInfo {
      protocol_version: ProtocolVersion::default(),
      server_info: Implementation {
        name: self.config.server.name.clone(),
        title: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
        icons: None,
        website_url: None,
      },
      capabilities: ServerCapabilities::builder()
        .enable_tools()
        .build(),
      instructions: Some("GitLab merge request review tools. Set GITLAB_URL (without /api/v4) and GITLAB_TOKEN before launch. Workflow: (1) get_merge_request for metadata and get_merge_request_changes for diff context; (2) get_merge_request_versions and take the first entry's base/head/start commit SHAs; (3) call create_merge_request_discussion with body markdown and a position JSON containing: base_sha, head_sha, start_sha, new_path, old_path, and line numbers (new_line for additions, old_line for deletions). The position_type field defaults to 'text' if not specified. Use create_merge_request_note for top-level MR comments.".to_string()),
    }
  }
}
