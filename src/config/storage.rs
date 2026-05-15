use crate::config::{ConnectConfig, MySqlConfig, PostgresConfig};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize, Default)]
struct StoredConfig {
    #[serde(default)]
    connections: StoredConnections,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct StoredConnections {
    #[serde(default)]
    postgres: Vec<StoredConnection>,
    #[serde(default)]
    mysql: Vec<StoredConnection>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredConnection {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    host: String,
    user: String,
    db_name: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

impl From<&PostgresConfig> for StoredConnection {
    fn from(cfg: &PostgresConfig) -> Self {
        Self {
            name: cfg.name.clone(),
            host: cfg.host.clone(),
            user: cfg.user.clone(),
            db_name: cfg.db_name.clone(),
            port: cfg.port,
            password: cfg.password.clone(),
        }
    }
}

impl From<&MySqlConfig> for StoredConnection {
    fn from(cfg: &MySqlConfig) -> Self {
        Self {
            name: cfg.name.clone(),
            host: cfg.host.clone(),
            user: cfg.user.clone(),
            db_name: cfg.db_name.clone(),
            port: cfg.port,
            password: cfg.password.clone(),
        }
    }
}

impl From<StoredConnection> for PostgresConfig {
    fn from(c: StoredConnection) -> Self {
        Self {
            name: c.name,
            host: c.host,
            user: c.user,
            db_name: c.db_name,
            port: c.port,
            password: c.password,
        }
    }
}

impl From<StoredConnection> for MySqlConfig {
    fn from(c: StoredConnection) -> Self {
        Self {
            name: c.name,
            host: c.host,
            user: c.user,
            db_name: c.db_name,
            port: c.port,
            password: c.password,
        }
    }
}

pub struct ConfigStorage;

impl ConfigStorage {
    /// Returns the default config file path: `~/.config/lazysql/config.toml`.
    /// Falls back to `./.config/lazysql/config.toml` when `HOME` is unset.
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("lazysql")
            .join("config.toml")
    }

    /// Reads saved connections from disk. Returns empty vec if file is missing or unparseable.
    pub fn load() -> Vec<ConnectConfig> {
        Self::load_from(&Self::config_path())
    }

    /// Writes connections to disk, creating directories as needed.
    pub fn save(connections: &[ConnectConfig]) -> Result<(), std::io::Error> {
        Self::save_to(&Self::config_path(), connections)
    }

    fn load_from(path: &Path) -> Vec<ConnectConfig> {
        let Ok(content) = fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(stored): Result<StoredConfig, _> = toml::from_str(&content) else {
            return Vec::new();
        };
        let pg = stored
            .connections
            .postgres
            .into_iter()
            .map(|c| ConnectConfig::Postgres(c.into()));
        let my = stored
            .connections
            .mysql
            .into_iter()
            .map(|c| ConnectConfig::MySql(c.into()));
        pg.chain(my).collect()
    }

    fn save_to(path: &Path, connections: &[ConnectConfig]) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut stored = StoredConfig::default();
        for c in connections {
            match c {
                ConnectConfig::Postgres(cfg) => stored.connections.postgres.push(cfg.into()),
                ConnectConfig::MySql(cfg) => stored.connections.mysql.push(cfg.into()),
            }
        }
        let content = toml::to_string(&stored)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, content)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::{MySqlConfig, PostgresConfig};

    #[test]
    fn load_returns_empty_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let result = ConfigStorage::load_from(&path);
        assert!(result.is_empty());
    }

    #[test]
    fn round_trip_postgres_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = ConnectConfig::Postgres(PostgresConfig {
            name: None,
            host: "localhost".to_string(),
            user: "alice".to_string(),
            db_name: "mydb".to_string(),
            port: 5432,
            password: Some("secret".to_string()),
        });

        ConfigStorage::save_to(&path, &[config]).unwrap();
        let loaded = ConfigStorage::load_from(&path);

        assert_eq!(loaded.len(), 1);
        let ConnectConfig::Postgres(cfg) = &loaded[0] else {
            panic!("Expected Postgres variant");
        };
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.user, "alice");
        assert_eq!(cfg.db_name, "mydb");
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.password, Some("secret".to_string()));
    }

    #[test]
    fn round_trip_no_password() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = ConnectConfig::Postgres(PostgresConfig {
            name: None,
            host: "db".to_string(),
            user: "bob".to_string(),
            db_name: "prod".to_string(),
            port: 5433,
            password: None,
        });

        ConfigStorage::save_to(&path, &[config]).unwrap();
        let loaded = ConfigStorage::load_from(&path);
        assert_eq!(loaded.len(), 1);
        let ConnectConfig::Postgres(cfg) = &loaded[0] else {
            panic!("Expected Postgres variant");
        };
        assert_eq!(cfg.password, None);
    }

    #[test]
    fn config_path_uses_lazysql_directory() {
        let path = ConfigStorage::config_path();
        assert!(path.ends_with(Path::new(".config").join("lazysql").join("config.toml")));
    }

    #[test]
    fn round_trip_mysql_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = ConnectConfig::MySql(MySqlConfig {
            name: Some("shop".to_string()),
            host: "localhost".to_string(),
            user: "root".to_string(),
            db_name: "shop".to_string(),
            port: 3307,
            password: Some("pw".to_string()),
        });

        ConfigStorage::save_to(&path, &[config]).unwrap();
        let loaded = ConfigStorage::load_from(&path);

        assert_eq!(loaded.len(), 1);
        let ConnectConfig::MySql(cfg) = &loaded[0] else {
            panic!("Expected MySql variant");
        };
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.port, 3307);
        assert_eq!(cfg.password, Some("pw".to_string()));
    }

    #[test]
    fn round_trip_mixed_drivers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let configs = vec![
            ConnectConfig::Postgres(PostgresConfig {
                name: None,
                host: "pghost".to_string(),
                user: "pguser".to_string(),
                db_name: "pgdb".to_string(),
                port: 5432,
                password: None,
            }),
            ConnectConfig::MySql(MySqlConfig {
                name: None,
                host: "myhost".to_string(),
                user: "myuser".to_string(),
                db_name: "mydb".to_string(),
                port: 3307,
                password: None,
            }),
        ];

        ConfigStorage::save_to(&path, &configs).unwrap();
        let loaded = ConfigStorage::load_from(&path);
        assert_eq!(loaded.len(), 2);
    }
}
