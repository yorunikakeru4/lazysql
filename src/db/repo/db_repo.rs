use crate::{
    config::ConnectConfig,
    db::{mysql::init::MySqlRepo, postgres::init::PostgresRepo},
};
use std::time::Duration;

/// Error type shared by database backends.
#[derive(Debug)]
pub enum DbError {
    /// Error returned by the PostgreSQL driver.
    Postgres(tokio_postgres::Error),
    /// Error returned by the MySQL driver.
    MySql(mysql_async::Error),
    /// Requested item was not found or cannot be loaded.
    NotFound(String),
    /// Database connection attempt exceeded the configured timeout.
    ConnectionTimeout(Duration),
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Postgres(e) => write!(f, "Postgres error: {e}"),
            DbError::MySql(e) => write!(f, "MySQL error: {e}"),
            DbError::NotFound(msg) => write!(f, "Not found: {msg}"),
            DbError::ConnectionTimeout(timeout) => {
                write!(f, "Connection timed out after {}s", timeout.as_secs())
            }
        }
    }
}

/// Active database client.
#[derive(Debug)]
pub enum DbClient {
    /// PostgreSQL client implementation.
    Postgres(PostgresRepo),
    /// MySQL client implementation.
    MySql(MySqlRepo),
}
impl DbClient {
    /// Opens a database client from a saved connection config.
    pub async fn new(connect_config: ConnectConfig) -> Result<Self, DbError> {
        match connect_config {
            ConnectConfig::Postgres(cfg) => {
                let repo = PostgresRepo::new(cfg).await?;
                Ok(DbClient::Postgres(repo))
            }
            ConnectConfig::MySql(cfg) => {
                let repo = MySqlRepo::new(cfg).await?;
                Ok(DbClient::MySql(repo))
            }
        }
    }
}
