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
    let (client, connection) = tokio_postgres::connect(&config.from(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}
#[cfg(test)]
mod test {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_connect() {
        let config = crate::config::PostgresConfig {
            host: "localhost".to_string(),
            user: "test_user".to_string(),
            db_name: "db_test".to_string(),
            port: 5432,
            password: Some("vBnA46MVSs".to_string()),
        };

        let client = PostgresRepo::new(config).await.unwrap();
        println!("Connection successful: {:?}", client);
    }
}
