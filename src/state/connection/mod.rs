pub mod form;
pub use form::{FIELD_LABELS, FormState};

use crate::config::Connect;

/// Health of a saved connection (probed on screen entry).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HealthStatus {
    #[default]
    Unknown,
    Live,
    Idle,
    Offline,
}

/// Display-only, database-agnostic view of a connection config.
#[derive(Debug, Clone)]
pub struct ConnectionMeta {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub db_name: String,
    pub user: String,
    pub driver: String,
}

impl From<&Connect> for ConnectionMeta {
    fn from(c: &Connect) -> Self {
        match c {
            Connect::Postgres(cfg) => ConnectionMeta {
                name: cfg.name.clone().unwrap_or_else(|| {
                    format!("{}:{}/{}", cfg.host, cfg.port, cfg.db_name)
                }),
                host: cfg.host.clone(),
                port: cfg.port,
                db_name: cfg.db_name.clone(),
                user: cfg.user.clone(),
                driver: "postgres".into(),
            },
        }
    }
}

/// Which pane has focus in the Database split view.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ActivePane {
    #[default]
    Schemas,
    Tables,
}

#[derive(Debug, Default)]
pub struct ConnectState {
    pub selected: usize,
    pub error: Option<String>,
    pub health: Vec<HealthStatus>,
}

impl ConnectState {
    /// Moves selection down, wrapping around when reaching the end.
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    /// Moves selection up, stopping at index 0.
    pub fn select_prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn select_next_wraps() {
        let mut state = ConnectState::default();
        state.selected = 1;
        state.select_next(2);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_prev_stops_at_zero() {
        let mut state = ConnectState::default();
        state.select_prev(3);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_noop_when_empty() {
        let mut state = ConnectState::default();
        state.select_next(0);
        state.select_prev(0);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn health_default_is_unknown() {
        assert_eq!(HealthStatus::default(), HealthStatus::Unknown);
    }

    #[test]
    fn connection_meta_name_fallback() {
        use crate::config::{Connect, PostgresConfig};
        let c = Connect::Postgres(PostgresConfig {
            name: None,
            host: "localhost".into(),
            port: 5432,
            user: "alice".into(),
            db_name: "mydb".into(),
            password: None,
        });
        let meta = ConnectionMeta::from(&c);
        assert_eq!(meta.name, "localhost:5432/mydb");
        assert_eq!(meta.driver, "postgres");
    }

    #[test]
    fn connection_meta_uses_explicit_name() {
        use crate::config::{Connect, PostgresConfig};
        let c = Connect::Postgres(PostgresConfig {
            name: Some("prod".into()),
            host: "db.prod.io".into(),
            port: 5432,
            user: "app".into(),
            db_name: "prod_db".into(),
            password: None,
        });
        let meta = ConnectionMeta::from(&c);
        assert_eq!(meta.name, "prod");
    }
}
