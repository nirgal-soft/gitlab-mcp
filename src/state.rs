use std::time::Instant;
use anyhow::{Context, Result};
use dotenvy::dotenv;
use crate::config::Config;
use crate::gitlab::GitLabClient;

#[cfg(feature = "database")]
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
  start_time: Instant,
  pub gitlab: GitLabClient,
  // Add your shared state here
  #[cfg(feature = "database")]
  pub db: Option<Arc<sqlx::SqlitePool>>,
}

impl ServerState {
  pub async fn new(_config: &Config) -> Result<Self> {
    dotenv().ok();

    let base_url = dotenvy::var("GITLAB_URL").context("GITLAB_URL environment variable is required")?;
    let token = dotenvy::var("GITLAB_TOKEN").context("GITLAB_TOKEN environment variable is required")?;
    let gitlab = GitLabClient::new(base_url, token)?;

    #[cfg(feature = "database")]
    let mut state = Self {
      start_time: Instant::now(),
      gitlab,
      db: None,
    };

    #[cfg(not(feature = "database"))]
    let state = Self {
      start_time: Instant::now(),
      gitlab,
    };

    #[cfg(feature = "database")]
    if let Some(db_config) = &_config.database {
      let pool = sqlx::SqlitePool::connect(&db_config.url).await?;
      state.db = Some(Arc::new(pool));
    }

    Ok(state)
  }

  pub fn uptime(&self) -> std::time::Duration {
    self.start_time.elapsed()
  }
}
