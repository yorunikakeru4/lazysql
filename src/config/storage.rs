use crate::config::{Connect, PostgresConfig};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
struct StoredConfig {
    connections: Vec<StoredConnection>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredConnection {
    host: String,
    user: String,
    db_name: String,
    port: u16,
    password: Option<String>,
}

pub struct ConfigStorage;

impl ConfigStorage {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("lazy-sql").join("config.toml")
    }

    /// Reads saved connections from disk. Returns empty vec if file is missing or unparseable.
    pub fn load() -> Vec<Connect> {
        let path = Self::config_path();
        let Ok(content) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        let Ok(stored): Result<StoredConfig, _> = toml::from_str(&content) else {
            return Vec::new();
        };
        stored
            .connections
            .into_iter()
            .map(|c| {
                Connect::Postgres(PostgresConfig {
                    host: c.host,
                    user: c.user,
                    db_name: c.db_name,
                    port: c.port,
                    password: c.password,
                })
            })
            .collect()
    }

    /// Writes connections to disk, creating directories as needed.
    pub fn save(connections: &[Connect]) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let stored = StoredConfig {
            connections: connections
                .iter()
                .map(|c| match c {
                    Connect::Postgres(cfg) => StoredConnection {
                        host: cfg.host.clone(),
                        user: cfg.user.clone(),
                        db_name: cfg.db_name.clone(),
                        port: cfg.port,
                        password: cfg.password.clone(),
                    },
                })
                .collect(),
        };
        if let Ok(content) = toml::to_string(&stored) {
            let _ = fs::write(&path, content);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn load_returns_empty_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }
        let result = ConfigStorage::load();
        assert!(result.is_empty());
        drop(dir);
    }

    #[test]
    fn round_trip_postgres_config() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let config = Connect::Postgres(PostgresConfig {
            host: "localhost".to_string(),
            user: "alice".to_string(),
            db_name: "mydb".to_string(),
            port: 5432,
            password: Some("secret".to_string()),
        });

        ConfigStorage::save(&[config]);
        let loaded = ConfigStorage::load();

        assert_eq!(loaded.len(), 1);
        let Connect::Postgres(cfg) = &loaded[0];
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.user, "alice");
        assert_eq!(cfg.db_name, "mydb");
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.password, Some("secret".to_string()));
        drop(dir);
    }

    #[test]
    fn round_trip_no_password() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let config = Connect::Postgres(PostgresConfig {
            host: "db".to_string(),
            user: "bob".to_string(),
            db_name: "prod".to_string(),
            port: 5433,
            password: None,
        });

        ConfigStorage::save(&[config]);
        let loaded = ConfigStorage::load();

        assert_eq!(loaded.len(), 1);
        let Connect::Postgres(cfg) = &loaded[0];
        assert_eq!(cfg.password, None);
        drop(dir);
    }
}
