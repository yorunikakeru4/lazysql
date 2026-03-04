use crate::{config::Connect, db::postgres::init::PostgresRepo};
#[derive(Debug)]
pub enum DbError {
    Postgres(tokio_postgres::Error),
    NotFound(String),
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Postgres(e) => write!(f, "Postgres error: {}", e),
            DbError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}
#[derive(Debug)]
pub enum DbClient {
    Postgres(PostgresRepo),
}
impl DbClient {
    pub async fn new(connect_config: Connect) -> Result<Self, DbError> {
        match connect_config {
            Connect::Postgres(cfg) => {
                let repo = PostgresRepo::new(cfg).await?;
                Ok(DbClient::Postgres(repo))
            }
        }
    }
}
/* #[async_trait]
pub trait Repo {
    async fn new(connect_config: Connect) -> Result<Self, DbError>
    where
        Self: Sized;
} */
