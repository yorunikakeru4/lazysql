use crate::config::PostgresConfig;
use crate::db::repo::db_repo::DbError;
use tokio_postgres::{Client, Error, NoTls};
#[derive(Debug)]
pub struct PostgresRepo {
    pub client: Client,
}
impl PostgresRepo {
    pub async fn new(connect_config: PostgresConfig) -> Result<Self, DbError> {
        let client = connect(connect_config).await.map_err(DbError::Postgres)?;
        Ok(PostgresRepo { client })
    }
}
async fn connect(config: PostgresConfig) -> Result<Client, Error> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    Ok(client)
}
#[cfg(test)]
mod test {
    use super::*;

    fn test_config() -> crate::config::PostgresConfig {
        crate::config::PostgresConfig {
            name: None,
            host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            user: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "test_user".to_string()),
            db_name: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "db_test".to_string()),
            port: std::env::var("TEST_DB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5439),
            password: std::env::var("TEST_DB_PASSWORD").ok(),
        }
    }

    #[tokio::test]
    async fn connect() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        println!("Connection successful: {:?}", client);
    }
}
