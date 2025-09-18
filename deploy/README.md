# Deployment Guide

This guide covers deploying your MCP server using Docker.

## Configuration Management

The MCP server uses a sophisticated configuration system that works across different environments:

### Configuration Priority (highest to lowest):
1. **Environment Variables** (MCP_* prefix)
2. **Docker Mount** (`/config.toml` in container)
3. **Local Override** (`./config.toml` in current directory)
4. **User Config** (`~/.config/gitlab-mcp/config.toml`)

### First Run Behavior:
- If no user config exists, the server creates `~/.config/gitlab-mcp/config.toml` with sane defaults
- If a local `config.toml` exists, it's copied to the user config directory
- The server will never overwrite existing user configurations

## Docker Deployment

### Quick Start

1. **Prepare your configuration:**
   ```bash
   # Copy one of the example configs
   cp streaming_config.toml.example config.toml
   # Edit config.toml with your settings
   ```

2. **Build and run:**
   ```bash
   cd deploy
   docker-compose up -d
   ```

### Configuration Options

The `docker-compose.yml` provides three configuration mounting options:

#### Option 1: Direct Mount (Default)
```yaml
volumes:
  - ../config.toml:/config.toml:ro
```
- Mounts your local `config.toml` directly into the container
- Changes require container restart
- Good for development and simple deployments

#### Option 2: User Config Directory
```yaml
volumes:
  - ~/.config/gitlab-mcp:/app/.config/gitlab-mcp:rw
```
- Mounts your user config directory into the container
- Allows the container to create/modify config files
- Preserves config changes across container restarts

#### Option 3: Named Volume (Recommended for Production)
```yaml
volumes:
  - gitlab-mcp-config:/app/.config/gitlab-mcp:rw
```
- Uses Docker managed volumes for configuration
- Persists across container recreation
- Best for production deployments

### Environment Variables

You can override any configuration value using environment variables:

```bash
# Set transport to HTTP streaming on port 9090
MCP_SERVER_TRANSPORT="{ http-streaming = { port = 9090 } }"

# Set log level to debug
MCP_TELEMETRY_LEVEL=debug

# Set server name
MCP_SERVER_NAME=my-custom-server
```

### Port Configuration

By default, the service exposes port 8080. To change this:

1. **Update your config.toml:**
   ```toml
   [server]
   transport = { http-streaming = { port = 3000 } }
   ```

2. **Update docker-compose.yml:**
   ```yaml
   ports:
     - "3000:3000"
   ```

### Commands

```bash
# Start the service
docker-compose up -d

# View logs
docker-compose logs -f gitlab-mcp

# Stop the service
docker-compose down

# Rebuild and restart
docker-compose up -d --build

# Clean up everything
docker-compose down --volumes --rmi local
```

### Troubleshooting

**Config not loading:**
- Check file permissions on mounted config files
- Verify config file syntax is valid TOML
- Check logs for configuration errors

**Permission issues:**
- The container runs as user `appuser` (not root)
- Ensure mounted files are readable by the container user
- Use `docker-compose exec gitlab-mcp ls -la /config.toml` to check

**Port conflicts:**
- Change the host port in `docker-compose.yml` if 8080 is already in use
- Ensure the container port matches your config.toml transport port