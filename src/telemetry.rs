use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::config::{TelemetryConfig, LogFormat};
use anyhow::Result;

pub fn init(config: &TelemetryConfig) -> Result<tracing_appender::non_blocking::WorkerGuard> {
  let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

  let (non_blocking, guard) = if let Some(file_path) = &config.file {
    let file_appender = tracing_appender::rolling::daily("logs", file_path);
    tracing_appender::non_blocking(file_appender)
  } else {
    tracing_appender::non_blocking(std::io::stdout())
  };

  let subscriber = tracing_subscriber::registry()
    .with(env_filter);

  match config.format {
    LogFormat::Pretty => {
      subscriber.with(tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .pretty())
        .init();
    }
    LogFormat::Json => {
      subscriber.with(tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .json())
        .init();
    }
  }

  Ok(guard)
}
