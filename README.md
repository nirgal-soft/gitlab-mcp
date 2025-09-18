# GitLab MCP Server

A Model Context Protocol (MCP) server that provides GitLab merge request review capabilities to AI assistants. This server enables AI assistants to fetch merge request details, review diffs, and create line-level discussions or general comments on GitLab merge requests.

## Features

- **Fetch MR Metadata** - Get title, author, state, approvals, and other merge request details
- **Review Diffs** - Retrieve file changes and diff hunks for code review
- **Line-Level Discussions** - Create precise code review comments on specific lines
- **General Comments** - Add top-level notes to merge requests
- **Multiple Transports** - Supports both stdio and HTTP streaming
- **Production Ready** - Structured logging, error handling, and Docker deployment

## Prerequisites

- Rust 1.75 or later
- GitLab personal access token with API access
- GitLab instance URL (self-hosted or gitlab.com)

## Installation

### From Source

```bash
# Clone the repository
git clone git@github.com:nirgal-soft/gitlab-mcp.git
cd gitlab-mcp

# Build the project
cargo build --release

# The binary will be at target/release/gitlab-mcp
```

### Using Docker

```bash
# Build the Docker image
docker build -t gitlab-mcp -f deploy/Dockerfile .

# Run with stdio transport
docker run -it \
  -e GITLAB_URL=https://gitlab.com \
  -e GITLAB_TOKEN=your-token \
  gitlab-mcp

# Run with HTTP streaming
docker run -p 8080:8080 \
  -e GITLAB_URL=https://gitlab.com \
  -e GITLAB_TOKEN=your-token \
  -v $(pwd)/streaming_config.toml:/config.toml:ro \
  gitlab-mcp
```

## Configuration

### Environment Variables

The server requires these environment variables:

```bash
# Required
export GITLAB_URL="https://gitlab.com"     # Your GitLab instance URL (without /api/v4)
export GITLAB_TOKEN="glpat-xxxxxxxxxxxx"   # Your GitLab personal access token

# Optional
export RUST_LOG="info"                     # Log level: debug, info, warn, error
```

### Configuration File

Create a `config.toml` file:

```toml
[server]
name = "gitlab-mcp"
# For stdio transport (default)
transport = "stdio"
# Or for HTTP streaming
# transport = { http-streaming = { port = 8080 } }

[telemetry]
level = "info"
format = "pretty"
# Optional: Log to file (required for stdio transport)
# file = "gitlab-mcp.log"
```

## Usage

### With Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "gitlab": {
      "command": "/path/to/gitlab-mcp",
      "args": [],
      "env": {
        "GITLAB_URL": "https://gitlab.com",
        "GITLAB_TOKEN": "your-token"
      }
    }
  }
}
```

### Available Tools

The server provides 5 tools for GitLab merge request operations:

#### 1. `get_merge_request`
Fetches merge request metadata including title, author, state, and approval status.

**Parameters:**
- `project`: Project ID or full path (e.g., "group/project")
- `merge_request_iid`: Merge request IID

#### 2. `get_merge_request_changes`
Retrieves the diff changes including file modifications and hunks.

**Parameters:**
- `project`: Project ID or full path
- `merge_request_iid`: Merge request IID

#### 3. `get_merge_request_versions`
Gets commit SHAs needed for creating line-level discussions.

**Parameters:**
- `project`: Project ID or full path
- `merge_request_iid`: Merge request IID

#### 4. `create_merge_request_discussion`
Creates a line-specific code review comment.

**Parameters:**
- `project`: Project ID or full path
- `merge_request_iid`: Merge request IID
- `body`: Markdown comment body
- `position`: Position object with:
  - `base_sha`, `head_sha`, `start_sha`: From `get_merge_request_versions`
  - `new_path`, `old_path`: File paths
  - `new_line`: For additions
  - `old_line`: For deletions
  - `position_type`: "text" (default) or "image"

#### 5. `create_merge_request_note`
Adds a general comment to the merge request.

**Parameters:**
- `project`: Project ID or full path
- `merge_request_iid`: Merge request IID
- `body`: Markdown comment body
- `confidential`: Optional, makes note visible only to project members

## Workflow Example

Here's the typical workflow for reviewing a merge request:

1. **Get MR metadata and changes:**
   ```
   get_merge_request(project="mygroup/myproject", merge_request_iid=123)
   get_merge_request_changes(project="mygroup/myproject", merge_request_iid=123)
   ```

2. **Get version info for discussions:**
   ```
   get_merge_request_versions(project="mygroup/myproject", merge_request_iid=123)
   ```
   Use the first version's SHAs for the next step.

3. **Create line-level discussion:**
   ```
   create_merge_request_discussion(
     project="mygroup/myproject",
     merge_request_iid=123,
     body="This line could be optimized",
     position={
       "base_sha": "abc123...",
       "head_sha": "def456...",
       "start_sha": "ghi789...",
       "new_path": "src/main.rs",
       "old_path": "src/main.rs",
       "new_line": 42
     }
   )
   ```

4. **Add general comment:**
   ```
   create_merge_request_note(
     project="mygroup/myproject",
     merge_request_iid=123,
     body="Overall the changes look good!"
   )
   ```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo test -- --nocapture

# Test with MCP Inspector
npx @modelcontextprotocol/inspector cargo run
```

### Building for Release

```bash
# Build optimized binary
cargo build --release

# Build Docker image
cd deploy
docker build -t gitlab-mcp:latest -f Dockerfile ..
```

## Architecture

The server is built with:
- **rmcp SDK** - MCP protocol implementation for Rust
- **tokio** - Async runtime
- **reqwest** - HTTP client for GitLab API
- **tracing** - Structured logging

Key components:
- `src/gitlab.rs` - GitLab API client
- `src/lib.rs` - MCP server implementation and tool routing
- `src/tools/gitlab.rs` - Tool request/response schemas
- `src/config.rs` - Configuration management
- `src/state.rs` - Server state and initialization

## Deployment

### Docker Compose

```yaml
services:
  gitlab-mcp:
    build:
      context: .
      dockerfile: deploy/Dockerfile
    image: gitlab-mcp:latest
    environment:
      - GITLAB_URL=https://gitlab.com
      - GITLAB_TOKEN=${GITLAB_TOKEN}
      - RUST_LOG=info
    ports:
      - "8080:8080"
    volumes:
      - ./config.toml:/config.toml:ro
    restart: unless-stopped
```

### Systemd Service

For systemd deployment, create a service file at `/etc/systemd/system/gitlab-mcp.service`:

```ini
[Unit]
Description=GitLab MCP Server
After=network.target

[Service]
Type=simple
User=mcp
Environment="GITLAB_URL=https://gitlab.com"
Environment="GITLAB_TOKEN=your-token"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/gitlab-mcp
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Troubleshooting

### Authentication Issues
- Ensure your GitLab token has `api` scope
- Check token hasn't expired
- Verify GitLab URL doesn't include `/api/v4` suffix

### Connection Problems
- For stdio: Check Claude Desktop config syntax
- For HTTP: Ensure port isn't already in use
- Enable debug logging: `RUST_LOG=debug`

### Tool Errors
- Verify project path or ID is correct
- Check merge request IID (not ID)
- For discussions, ensure you get versions first

## License

MIT License - see [license.txt](license.txt)

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Repository

- **GitHub**: https://github.com/nirgal-soft/gitlab-mcp
- **Issues**: https://github.com/nirgal-soft/gitlab-mcp/issues