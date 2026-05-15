pub mod form;
pub use form::{FIELD_LABELS, FormState};

use crate::config::ConnectConfig;

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

/// Reachability state for a saved connection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection has not been checked yet.
    #[default]
    Unknown,
    /// Last connection check succeeded.
    Online,
    /// Last connection check failed or timed out.
    Offline,
}

impl From<&ConnectConfig> for ConnectionMeta {
    fn from(c: &ConnectConfig) -> Self {
        match c {
            ConnectConfig::Postgres(cfg) => ConnectionMeta {
                name: cfg.name.clone().unwrap_or_else(|| {
                    let host = &cfg.host;
                    let port = cfg.port;
                    let db_name = &cfg.db_name;
                    format!("{host}:{port}/{db_name}")
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
    /// Whether the inline add/edit connection form is open on the Connections screen.
    pub form_open: bool,
    /// Last reachability result for the unsaved connection form draft.
    pub draft_status: Option<ConnectionStatus>,
}

impl ConnectState {
    /// Opens the inline connection form and clears draft-only state.
    pub fn open_form(&mut self) {
        self.form_open = true;
        self.draft_status = None;
        self.error = None;
    }

    /// Closes the inline connection form and clears draft-only state.
    pub fn close_form(&mut self) {
        self.form_open = false;
        self.draft_status = None;
    }

    /// Moves selection down, wrapping around when reaching the end.
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    /// Moves selection up, wrapping around when reaching the start.
    pub fn select_prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            len - 1
        } else {
            self.selected - 1
        };
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
    fn select_prev_wraps_from_zero_to_last_item() {
        let mut state = ConnectState::default();
        state.select_prev(3);
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn select_noop_when_empty() {
        let mut state = ConnectState::default();
        state.select_next(0);
        state.select_prev(0);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn default_inline_form_state_is_closed_without_draft_status() {
        let state = ConnectState::default();

        assert!(!state.form_open);
        assert_eq!(state.draft_status, None);
    }

    #[test]
    fn close_form_resets_inline_form_flags() {
        let mut state = ConnectState::default();
        state.form_open = true;
        state.draft_status = Some(ConnectionStatus::Offline);

        state.close_form();

        assert!(!state.form_open);
        assert_eq!(state.draft_status, None);
    }

    #[test]
    fn connection_meta_name_fallback() {
        use crate::config::{ConnectConfig, PostgresConfig};
        let c = ConnectConfig::Postgres(PostgresConfig {
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
        use crate::config::{ConnectConfig, PostgresConfig};
        let c = ConnectConfig::Postgres(PostgresConfig {
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
