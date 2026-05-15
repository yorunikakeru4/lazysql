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
