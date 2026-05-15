use crate::config::MySqlConfig;
use crate::db::repo::db_repo::DbError;

/// MySQL database connection wrapper.
#[derive(Clone)]
pub struct MySqlRepo {
    conn: std::sync::Arc<tokio::sync::Mutex<mysql_async::Conn>>,
}

impl std::fmt::Debug for MySqlRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlRepo").finish_non_exhaustive()
    }
}

impl MySqlRepo {
    /// Opens a new MySQL connection from the given config.
    pub async fn new(cfg: MySqlConfig) -> Result<Self, DbError> {
        let opts = mysql_async::OptsBuilder::default()
            .ip_or_hostname(cfg.host)
            .tcp_port(cfg.port)
            .user(Some(cfg.user))
            .pass(cfg.password)
            .db_name(Some(cfg.db_name));
        let conn = mysql_async::Conn::new(opts).await.map_err(DbError::MySql)?;
        Ok(MySqlRepo {
            conn: std::sync::Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_config() -> crate::config::MySqlConfig {
        crate::config::MySqlConfig {
            name: None,
            host: std::env::var("TEST_MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string()),
            user: std::env::var("TEST_MYSQL_USER").unwrap_or_else(|_| "test_user".to_string()),
            db_name: std::env::var("TEST_MYSQL_DB").unwrap_or_else(|_| "db_test".to_string()),
            port: std::env::var("TEST_MYSQL_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3307),
            password: std::env::var("TEST_MYSQL_PASSWORD").ok(),
        }
    }

    #[tokio::test]
    async fn connect() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        println!("MySQL connection successful: {:?}", repo);
    }
}
