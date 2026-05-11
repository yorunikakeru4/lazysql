pub mod storage;

#[derive(Debug, Clone)]
pub enum Connect {
    Postgres(PostgresConfig),
    // MySql(MySqlConfig),
    // Sqlite(SqliteConfig),
}

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub user: String,
    pub db_name: String,
    pub port: u16,
    pub password: Option<String>,
}
impl PostgresConfig {
    /// Builds a libpq-style connection string. Password is included only when set.
    pub fn connection_string(&self) -> String {
        let base = format!(
            "host={} user={} dbname={} port={}",
            self.host, self.user, self.db_name, self.port
        );
        match &self.password {
            Some(pw) => format!("{} password={}", base, pw),
            None => base,
        }
    }
}
