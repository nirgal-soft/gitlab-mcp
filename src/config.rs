use serde::Deserialize;
use config::{Config as ConfigBuilder, ConfigError, File};
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
  pub server: ServerConfig,
  pub telemetry: TelemetryConfig,
  #[cfg(feature = "auth")]
  pub redis: Option<RedisConfig>,
  #[cfg(feature = "database")]
  pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
  pub name: String,
  pub transport: TransportType,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
  Stdio,
  #[serde(rename = "http-streaming")]
  HttpStreaming { port: u16 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
  pub level: String,
  pub format: LogFormat,
  pub file: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
  Pretty,
  Json,
}

#[cfg(feature = "auth")]
#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
  pub url: String,
}

#[cfg(feature = "database")]
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
  pub url: String,
  pub max_connections: u32,
}

impl Config {
  pub fn load() -> Result<Self, ConfigError> {
    // Check for --http-port argument
    let args: Vec<String> = std::env::args().collect();
    let mut http_port: Option<u16> = None;

    for i in 0..args.len() {
      if args[i] == "--http-port" && i + 1 < args.len() {
        if let Ok(port) = args[i + 1].parse::<u16>() {
          http_port = Some(port);
        }
      }
    }

    // Check for config files
    let config_path = if Path::new("config.toml").exists() {
      Some("config.toml")
    } else if Path::new("/config.toml").exists() {
      Some("/config.toml")
    } else {
      None
    };

    // If we have a config file, use it
    if let Some(path) = config_path {
      tracing::info!("Loading config from: {}", path);
      let config = ConfigBuilder::builder()
        .add_source(File::with_name(path))
        .build()?;

      let mut config: Config = config.try_deserialize()?;

      // Force logging to file for stdio transport
      if matches!(config.server.transport, TransportType::Stdio) && config.telemetry.file.is_none() {
        config.telemetry.file = Some(format!("/tmp/{}.log", env!("CARGO_PKG_NAME")));
      }

      return Ok(config);
    }

    // No config file - build from defaults/environment
    let transport = if let Some(port) = http_port {
      tracing::info!("No config file found, using HTTP streaming on port {} (from --http-port)", port);
      TransportType::HttpStreaming { port }
    } else if let Ok(port_str) = std::env::var("PORT") {
      if let Ok(port) = port_str.parse::<u16>() {
        tracing::info!("No config file found, using HTTP streaming on port {} (from PORT env)", port);
        TransportType::HttpStreaming { port }
      } else {
        tracing::info!("No config file found, using default stdio configuration");
        TransportType::Stdio
      }
    } else {
      tracing::info!("No config file found, using default stdio configuration");
      TransportType::Stdio
    };

    // Set log file for stdio transport
    let log_file = if matches!(transport, TransportType::Stdio) {
      Some(format!("/tmp/{}.log", env!("CARGO_PKG_NAME")))
    } else {
      None
    };

    Ok(Config {
      server: ServerConfig {
        name: env!("CARGO_PKG_NAME").to_string(),
        transport,
      },
      telemetry: TelemetryConfig {
        level: std::env::var("MCP_TELEMETRY_LEVEL").unwrap_or_else(|_| "info".to_string()),
        format: match std::env::var("MCP_TELEMETRY_FORMAT").as_deref() {
          Ok("json") => LogFormat::Json,
          _ => LogFormat::Pretty,
        },
        file: log_file,
      },
      #[cfg(feature = "auth")]
      redis: std::env::var("MCP_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .ok()
        .map(|url| RedisConfig { url }),
      #[cfg(feature = "database")]
      database: std::env::var("DATABASE_URL").ok().map(|url| DatabaseConfig {
        url,
        max_connections: 10,
      }),
    })
  }
}
