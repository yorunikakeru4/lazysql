use async_trait::async_trait;
#[derive(Debug)]
pub enum DbError {
    Postgres(tokio_postgres::Error),
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Postgres(e) => write!(f, "Postgres error: {}", e),
        }
    }
}
#[derive(Debug)]
pub enum DbClient {
    Postgres(tokio_postgres::Client),
}
#[async_trait]
pub trait Repo {
    async fn new(connect_config: crate::config::Connect) -> Result<Self, DbError>
    where
        Self: Sized;
}
