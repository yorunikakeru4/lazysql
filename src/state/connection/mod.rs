pub mod form;
pub use form::{FIELD_LABELS, FormState};

use crate::{config::ConnectConfig, state::picker::PickerState};

/// Stable identifier for supported database drivers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    /// PostgreSQL direct connection.
    #[default]
    Postgres,
    /// MySQL direct connection.
    MySql,
}

impl DriverKind {
    /// Returns the display label for this driver.
    pub fn label(self) -> &'static str {
        self.definition().label
    }

    /// Returns the default port for this driver.
    pub fn default_port(self) -> u16 {
        self.definition().default_port
    }

    fn definition(self) -> &'static DriverDefinition {
        DRIVER_REGISTRY
            .iter()
            .find(|driver| driver.kind == self)
            .expect("driver kind must be registered")
    }
}

/// Metadata used to render and select database drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverDefinition {
    /// Stable machine id for the driver.
    pub id: &'static str,
    /// Driver kind used by config builders.
    pub kind: DriverKind,
    /// User-facing short label.
    pub label: &'static str,
    /// Short capability summary.
    pub summary: &'static str,
    /// Terms included in fuzzy filtering.
    pub aliases: &'static [&'static str],
    /// Default TCP port.
    pub default_port: u16,
}

/// Registry for drivers supported by the add/edit connection UI.
pub const DRIVER_REGISTRY: &[DriverDefinition] = &[
    DriverDefinition {
        id: "postgres",
        kind: DriverKind::Postgres,
        label: "postgres",
        summary: "libpq compatible",
        aliases: &["pg", "postgresql"],
        default_port: 5432,
    },
    DriverDefinition {
        id: "mysql",
        kind: DriverKind::MySql,
        label: "mysql",
        summary: "mysql_async",
        aliases: &["my", "mariadb"],
        default_port: 3306,
    },
];
/// Returns drivers matching the current query.
pub fn filtered_drivers(query: &str) -> Vec<&'static DriverDefinition> {
    let query = query.trim().to_ascii_lowercase();
    DRIVER_REGISTRY
        .iter()
        .filter(|driver| {
            if query.is_empty() {
                return true;
            }
            driver.id.contains(&query)
                || driver.label.contains(&query)
                || driver
                    .aliases
                    .iter()
                    .any(|alias| alias.to_ascii_lowercase().contains(&query))
        })
        .collect()
}
/// Returns the selected driver from the filtered list.
pub fn selected_driver(query: &str, selected: usize) -> Option<&'static DriverDefinition> {
    filtered_drivers(query).get(selected).copied()
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
            ConnectConfig::MySql(cfg) => ConnectionMeta {
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
                driver: "mysql".into(),
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
    /// Whether the driver picker is open on the Connections screen.
    pub driver_picker_open: bool,
    /// State for the add-connection driver picker.
    pub driver_picker: PickerState,
    /// Last reachability result for the unsaved connection form draft.
    pub draft_status: Option<ConnectionStatus>,
}

impl ConnectState {
    /// Opens the driver picker and clears draft-only state.
    pub fn open_driver_picker(&mut self) {
        self.driver_picker_open = true;
        self.form_open = false;
        self.draft_status = None;
        self.error = None;
    }

    /// Closes the driver picker.
    pub fn close_driver_picker(&mut self) {
        self.driver_picker_open = false;
        self.driver_picker.reset();
    }

    /// Opens the inline connection form and clears draft-only state.
    pub fn open_form(&mut self) {
        self.form_open = true;
        self.driver_picker_open = false;
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

    #[test]
    fn driver_registry_exposes_supported_drivers() {
        let ids: Vec<&str> = DRIVER_REGISTRY.iter().map(|driver| driver.id).collect();

        assert_eq!(ids, vec!["postgres", "mysql"]);
    }

    #[test]
    fn driver_kind_uses_registry_defaults() {
        assert_eq!(DriverKind::Postgres.label(), "postgres");
        assert_eq!(DriverKind::Postgres.default_port(), 5432);
        assert_eq!(DriverKind::MySql.label(), "mysql");
        assert_eq!(DriverKind::MySql.default_port(), 3306);
    }

    #[test]
    fn driver_picker_filters_by_label_and_aliases() {
        let picker = PickerState {
            query: "pg".to_string(),
            ..Default::default()
        };

        let filtered = filtered_drivers(picker.query.as_str());

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].kind, DriverKind::Postgres);
    }

    #[test]
    fn driver_picker_selection_wraps_filtered_results() {
        let mut picker = PickerState::default();

        let filtered = filtered_drivers(picker.query.as_str());
        picker.move_prev();
        assert_eq!(
            selected_driver(&picker.query, picker.selected)
                .unwrap()
                .kind,
            DriverKind::Postgres
        );

        picker.move_next(filtered.len());
        assert_eq!(
            selected_driver(&picker.query, picker.selected)
                .unwrap()
                .kind,
            DriverKind::MySql
        );
    }

    #[test]
    fn connect_state_opens_driver_picker_before_form() {
        let mut state = ConnectState::default();

        state.open_driver_picker();

        assert!(state.driver_picker_open);
        assert!(!state.form_open);
        assert_eq!(
            selected_driver(&state.driver_picker.query, state.driver_picker.selected)
                .unwrap()
                .kind,
            DriverKind::Postgres
        );
    }
}
