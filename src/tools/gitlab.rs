use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MergeRequestLocator {
  /// Project ID or full path (e.g. "group/project")
  pub project: String,
  /// Merge request IID
  pub merge_request_iid: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMergeRequestRequest {
  #[serde(flatten)]
  pub locator: MergeRequestLocator,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMergeRequestChangesRequest {
  #[serde(flatten)]
  pub locator: MergeRequestLocator,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMergeRequestVersionsRequest {
  #[serde(flatten)]
  pub locator: MergeRequestLocator,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateMergeRequestDiscussionRequest {
  #[serde(flatten)]
  pub locator: MergeRequestLocator,
  /// Markdown body of the discussion comment
  pub body: String,
  /// Position payload for line-specific comments
  pub position: Value,
  /// Optionally resolve the discussion immediately
  #[serde(default)]
  pub resolve: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateMergeRequestNoteRequest {
  #[serde(flatten)]
  pub locator: MergeRequestLocator,
  /// Markdown body of the note
  pub body: String,
  /// Create a confidential note (visible only to project members with access)
  #[serde(default)]
  pub confidential: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionPositionType {
  Text,
  Image,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionLinePositionType {
  New,
  Old,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DiscussionLineReference {
  pub line_code: String,
  #[serde(rename = "type")]
  pub position_type: DiscussionLinePositionType,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub old_line: Option<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub new_line: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DiscussionLineRange {
  pub start: DiscussionLineReference,
  pub end: DiscussionLineReference,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DiscussionPosition {
  pub base_sha: String,
  pub head_sha: String,
  pub start_sha: String,
  #[serde(default = "default_position_type")]
  pub position_type: DiscussionPositionType,
  pub new_path: String,
  pub old_path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub new_line: Option<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub old_line: Option<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub line_range: Option<DiscussionLineRange>,
}

fn default_position_type() -> DiscussionPositionType {
  DiscussionPositionType::Text
}

impl DiscussionPosition {
  pub fn validate(&self) -> Result<(), McpError> {
    if self.base_sha.trim().is_empty()
      || self.head_sha.trim().is_empty()
      || self.start_sha.trim().is_empty()
    {
      return Err(McpError::invalid_params(
        "GitLab discussion position requires base_sha, head_sha, and start_sha",
        None,
      ));
    }

    if self.new_path.trim().is_empty() || self.old_path.trim().is_empty() {
      return Err(McpError::invalid_params(
        "GitLab discussion position requires both new_path and old_path",
        None,
      ));
    }

    let has_line = self.new_line.is_some() || self.old_line.is_some() || self.line_range.is_some();
    if !has_line {
      return Err(McpError::invalid_params(
        "GitLab discussion position requires at least one of new_line, old_line, or line_range",
        None,
      ));
    }

    Ok(())
  }
}

fn parse_discussion_position(raw: &Value) -> Result<DiscussionPosition, McpError> {
  let value = match raw {
    Value::String(s) => serde_json::from_str::<Value>(s).map_err(|err| {
      McpError::invalid_params("position string is not valid JSON", Some(Value::String(err.to_string())))
    })?,
    other => other.clone(),
  };

  serde_json::from_value::<DiscussionPosition>(value).map_err(|err| {
    McpError::invalid_params(
      "position must be a GitLab discussion position object",
      Some(Value::String(err.to_string())),
    )
  })
}

pub fn map_to_payload(map: Map<String, Value>) -> Value {
  Value::Object(map)
}

pub fn discussion_payload(req: &CreateMergeRequestDiscussionRequest) -> Result<Value, McpError> {
  let position = parse_discussion_position(&req.position)?;
  position.validate()?;

  let mut map = Map::new();
  map.insert("body".to_string(), Value::String(req.body.clone()));
  let position = serde_json::to_value(&position).map_err(|err| {
    McpError::internal_error(
      "Failed to serialize GitLab discussion position",
      Some(Value::String(err.to_string())),
    )
  })?;
  map.insert("position".to_string(), position);
  if let Some(resolve) = req.resolve {
    map.insert("resolve".to_string(), Value::Bool(resolve));
  }
  Ok(map_to_payload(map))
}

pub fn note_payload(req: &CreateMergeRequestNoteRequest) -> Value {
  let mut map = Map::new();
  map.insert("body".to_string(), Value::String(req.body.clone()));
  if let Some(confidential) = req.confidential {
    map.insert("confidential".to_string(), Value::Bool(confidential));
  }
  map_to_payload(map)
}

pub fn json_result(value: Value) -> Result<CallToolResult, McpError> {
  serde_json::to_string_pretty(&value)
    .map(|text| CallToolResult::success(vec![Content::text(text)]))
    .map_err(|err| McpError::internal_error(
      "Failed to format GitLab response",
      Some(Value::String(err.to_string())),
    ))
}
