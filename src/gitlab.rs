use anyhow::Result;
use reqwest::{Client, StatusCode};
use rmcp::model::ErrorData as McpError;
use serde_json::Value;
use urlencoding::encode;

#[derive(Clone)]
pub struct GitLabClient {
  base_url: String,
  token: String,
  http: Client,
}

impl GitLabClient {
  pub fn new(base_url: String, token: String) -> Result<Self> {
    if base_url.trim().is_empty() {
      anyhow::bail!("GITLAB_URL environment variable is empty");
    }
    if token.trim().is_empty() {
      anyhow::bail!("GITLAB_TOKEN environment variable is empty");
    }

    let http = Client::builder()
      .user_agent("gitlab-mcp/0.1")
      .build()?;

    let trimmed = base_url.trim_end_matches('/');
    let base_url = if trimmed.ends_with("/api/v4") {
      trimmed.to_string()
    } else if trimmed.ends_with("/api") {
      format!("{}/v4", trimmed)
    } else {
      format!("{}/api/v4", trimmed)
    };

    Ok(Self {
      base_url,
      token,
      http,
    })
  }

  fn projects_base(&self, project: &str) -> String {
    format!("{}/projects/{}", self.base_url, encode(project))
  }

  async fn handle_response(response: reqwest::Response) -> Result<Value, McpError> {
    let status = response.status();
    let text = response.text().await.map_err(|err| {
      McpError::internal_error(
        "Failed to read GitLab response body",
        Some(Value::String(err.to_string())),
      )
    })?;

    if status.is_success() {
      serde_json::from_str(&text).map_err(|err| {
        McpError::internal_error(
          "GitLab returned invalid JSON",
          Some(Value::String(err.to_string())),
        )
      })
    } else {
      let detail = if text.is_empty() {
        Value::String(status.canonical_reason().unwrap_or("Unknown GitLab error").to_string())
      } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
      };
      let error = match status {
        StatusCode::NOT_FOUND => {
          McpError::invalid_params("GitLab resource not found", Some(detail.clone()))
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
          McpError::invalid_request("GitLab authentication failed", Some(detail.clone()))
        }
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
          McpError::invalid_params("GitLab reported a validation error", Some(detail.clone()))
        }
        _ => McpError::internal_error("GitLab request failed", Some(detail)),
      };

      Err(error)
    }
  }

  async fn send_get(&self, url: String) -> Result<Value, McpError> {
    let response = self.http
      .get(&url)
      .header("PRIVATE-TOKEN", &self.token)
      .send()
      .await
      .map_err(|err| McpError::internal_error(
        "Failed to reach GitLab",
        Some(Value::String(err.to_string())),
      ))?;

    Self::handle_response(response).await
  }

  async fn send_post(&self, url: String, payload: Value) -> Result<Value, McpError> {
    let response = self.http
      .post(&url)
      .header("PRIVATE-TOKEN", &self.token)
      .json(&payload)
      .send()
      .await
      .map_err(|err| McpError::internal_error(
        "Failed to reach GitLab",
        Some(Value::String(err.to_string())),
      ))?;

    Self::handle_response(response).await
  }

  pub async fn get_merge_request(&self, project: &str, merge_request_iid: u64) -> Result<Value, McpError> {
    let url = format!(
      "{}/merge_requests/{}",
      self.projects_base(project),
      merge_request_iid
    );
    self.send_get(url).await
  }

  pub async fn get_merge_request_changes(&self, project: &str, merge_request_iid: u64) -> Result<Value, McpError> {
    let url = format!(
      "{}/merge_requests/{}/changes",
      self.projects_base(project),
      merge_request_iid
    );
    self.send_get(url).await
  }

  pub async fn get_merge_request_versions(&self, project: &str, merge_request_iid: u64) -> Result<Value, McpError> {
    let url = format!(
      "{}/merge_requests/{}/versions",
      self.projects_base(project),
      merge_request_iid
    );
    self.send_get(url).await
  }

  pub async fn create_merge_request_discussion(
    &self,
    project: &str,
    merge_request_iid: u64,
    payload: Value,
  ) -> Result<Value, McpError> {
    let url = format!(
      "{}/merge_requests/{}/discussions",
      self.projects_base(project),
      merge_request_iid
    );
    self.send_post(url, payload).await
  }

  pub async fn create_merge_request_note(
    &self,
    project: &str,
    merge_request_iid: u64,
    payload: Value,
  ) -> Result<Value, McpError> {
    let url = format!(
      "{}/merge_requests/{}/notes",
      self.projects_base(project),
      merge_request_iid
    );
    self.send_post(url, payload).await
  }
}
