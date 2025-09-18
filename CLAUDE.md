# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Development
```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run the server with stdio transport (default)
cargo run

# Run the server with HTTP streaming transport
cargo run -- --http-port 8080

# Run with custom config file
cargo run -- --config my-config.toml

# Enable debug logging
RUST_LOG=debug cargo run
```

### Testing and Quality
```bash
# Run all tests
cargo test

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check
```

### Configuration
Before running the server, set the required environment variables:
```bash
export GITLAB_URL="https://gitlab.example.com"  # Without /api/v4
export GITLAB_TOKEN="your-gitlab-personal-access-token"
```

## Architecture

### Core Components

**MCP Server Implementation (`src/lib.rs`)**
- Implements the Model Context Protocol server using the `rmcp` SDK
- Supports both stdio and HTTP streaming transports
- Uses the `#[tool_router]` and `#[tool]` macros to define GitLab MR review tools
- All tools are async and return `CallToolResult` with formatted JSON responses

**GitLab Client (`src/gitlab.rs`)**
- HTTP client wrapper for GitLab API v4
- Handles authentication via personal access tokens
- Provides methods for merge request operations (get metadata, changes, versions, discussions, notes)
- Automatically normalizes GitLab URLs and handles API path construction
- Comprehensive error mapping from HTTP status codes to MCP errors

**Tool Definitions (`src/tools/gitlab.rs`)**
- Defines request/response schemas using `schemars` for MCP tool parameters
- `DiscussionPosition` struct validates GitLab discussion position requirements
- Handles both JSON object and JSON string formats for position data
- Provides payload builders for API requests

### GitLab Integration Tools

The server provides 5 GitLab merge request tools:

1. **`get_merge_request`**: Fetches MR metadata (title, author, state, approvals)
2. **`get_merge_request_changes`**: Retrieves diff/changes for code review
3. **`get_merge_request_versions`**: Gets commit SHAs needed for line-level discussions
4. **`create_merge_request_discussion`**: Creates line-specific code review comments
5. **`create_merge_request_note`**: Adds general top-level MR comments

### Line-Level Discussion Workflow

For creating line-specific discussions on GitLab MRs:
1. First call `get_merge_request_versions` to obtain base/head/start SHAs
2. Use the first version entry's commit SHAs
3. Create discussion with position object containing:
   - `base_sha`, `head_sha`, `start_sha` (from step 1)
   - `new_path` and `old_path` (file paths)
   - Line markers: `new_line` for additions, `old_line` for deletions, both for context
   - Optional `line_range` for multi-line comments

### State Management (`src/state.rs`)
- Holds shared server state including GitLab client instance
- Initialized once at server startup with validated configuration

### Transport Options
- **stdio**: Default mode for direct JSON-RPC communication
- **HTTP Streaming**: Uses rmcp's `StreamableHttpService` with local session management