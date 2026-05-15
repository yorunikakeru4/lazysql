pub mod storage;

#[derive(Debug, Clone)]
pub enum ConnectConfig {
    Postgres(PostgresConfig),
    MySql(MySqlConfig),
}

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub name: Option<String>,
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
            Some(pw) => format!("{base} password={pw}"),
            None => base,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MySqlConfig {
    pub name: Option<String>,
    pub host: String,
    pub user: String,
    pub db_name: String,
    pub port: u16,
    pub password: Option<String>,
}
impl MySqlConfig {
    /// Builds a mysql_async-compatible URL. Password included only when set.
    pub fn url(&self) -> String {
        match &self.password {
            Some(pw) => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.user, pw, self.host, self.port, self.db_name
            ),
            None => format!(
                "mysql://{}@{}:{}/{}",
                self.user, self.host, self.port, self.db_name
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mysql_url_with_password() {
        let cfg = MySqlConfig {
            name: None,
            host: "localhost".to_string(),
            user: "root".to_string(),
            db_name: "shop".to_string(),
            port: 3307,
            password: Some("secret".to_string()),
        };
        assert_eq!(cfg.url(), "mysql://root:secret@localhost:3307/shop");
    }

    #[test]
    fn mysql_url_without_password() {
        let cfg = MySqlConfig {
            name: None,
            host: "db".to_string(),
            user: "alice".to_string(),
            db_name: "mydb".to_string(),
            port: 3306,
            password: None,
        };
        assert_eq!(cfg.url(), "mysql://alice@db:3306/mydb");
    }
}
