use crate::{config::ConnectConfig, db::postgres::init::PostgresRepo};
use std::time::Duration;

/// Error type shared by database backends.
#[derive(Debug)]
pub enum DbError {
    /// Error returned by the PostgreSQL driver.
    Postgres(tokio_postgres::Error),
    /// Requested item was not found or cannot be loaded.
    NotFound(String),
    /// Database connection attempt exceeded the configured timeout.
    ConnectionTimeout(Duration),
    // ConfigError(String),
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Postgres(e) => write!(f, "Postgres error: {e}"),
            DbError::NotFound(msg) => write!(f, "Not found: {msg}"),
            DbError::ConnectionTimeout(timeout) => {
                let seconds = timeout.as_secs();
                write!(f, "Connection timed out after {seconds}s")
            } // DbError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}
/// Active database client.
#[derive(Debug)]
pub enum DbClient {
    /// PostgreSQL client implementation.
    Postgres(PostgresRepo),
}
impl DbClient {
    /// Opens a database client from a saved connection config.
    pub async fn new(connect_config: ConnectConfig) -> Result<Self, DbError> {
        match connect_config {
            ConnectConfig::Postgres(cfg) => {
                let repo = PostgresRepo::new(cfg).await?;
                Ok(DbClient::Postgres(repo))
            }
            ConnectConfig::MySql(_) => {
                todo!("MySQL support not yet implemented")
            }
        }
    }
}
