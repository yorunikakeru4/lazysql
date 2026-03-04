pub mod storage;
#[derive(Debug)]
pub enum DbKind {
    Postgres,
    /* MySql,
    Sqlite, */
}
#[derive(Debug, Clone)]
pub enum Connect {
    Postgres(PostgresConfig),
    // MySql(MySqlConfig),
    // Sqlite(SqliteConfig),
}

impl Connect {
    pub fn kind(&self) -> DbKind {
        match self {
            Connect::Postgres(_) => DbKind::Postgres,
            /* Connect::MySql(_) => DbKind::MySql,
            Connect::Sqlite(_) => DbKind::Sqlite, */
        }
    }
}
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub user: String,
    pub db_name: String,
    pub port: u32,
    pub password: Option<String>,
}
impl PostgresConfig {
    // WARNING: это формат для postgre, если нужно будет менять на другой драйвер, то нужно будет изменить формат строки подключения (наверное, для mysql будет другой формат)
    pub fn from(&self) -> String {
        let con = format!(
            "host={} user={} dbname={} port={}",
            self.host, self.user, self.db_name, self.port
        );
        if let Some(password) = &self.password {
            format!("{} password={}", con, password)
        } else {
            con
        }
    }
}
