use crate::config::{ConnectConfig, MySqlConfig, PostgresConfig};
use crate::state::connection::DriverKind;

const FIELD_COUNT: usize = 6;

pub const FIELD_LABELS: [&str; FIELD_COUNT] = [
    "Name (optional)",
    "Host",
    "Port",
    "User",
    "Database",
    "Password (optional)",
];

/// Form state for the Add Connection screen.
#[derive(Debug, Default)]
pub struct FormState {
    /// Driver selected for this form.
    pub driver: DriverKind,
    /// Input buffers: [name, host, port, user, db_name, password].
    pub values: [String; FIELD_COUNT],
    /// Index of the currently focused field (0..FIELD_COUNT).
    pub focused: usize,
    /// Validation error shown at the bottom of the form.
    pub error: Option<String>,
    /// Selected connection index when editing an existing connection.
    pub editing_index: Option<usize>,
}

impl FormState {
    /// Builds an empty form with driver-specific defaults.
    pub fn new_for_driver(driver: DriverKind) -> Self {
        let mut form = Self {
            driver,
            ..Default::default()
        };
        form.values[2] = driver.default_port().to_string();
        form
    }

    /// Moves focus to the next field, wrapping around.
    pub fn next_field(&mut self) {
        self.focused = (self.focused + 1) % FIELD_COUNT;
    }

    /// Moves focus to the previous field, wrapping around.
    pub fn prev_field(&mut self) {
        self.focused = (self.focused + FIELD_COUNT - 1) % FIELD_COUNT;
    }

    /// Returns a mutable reference to the currently focused field's buffer.
    pub fn current_value_mut(&mut self) -> &mut String {
        &mut self.values[self.focused]
    }

    /// Resets all fields and error to their defaults.
    pub fn reset(&mut self) {
        *self = FormState::default();
    }

    /// Builds a pre-filled form for editing an existing Postgres connection.
    pub fn from_postgres_config(cfg: &PostgresConfig, editing_index: Option<usize>) -> Self {
        let mut form = Self {
            driver: DriverKind::Postgres,
            editing_index,
            ..Default::default()
        };
        form.values[0] = cfg.name.clone().unwrap_or_default();
        form.values[1] = cfg.host.clone();
        form.values[2] = cfg.port.to_string();
        form.values[3] = cfg.user.clone();
        form.values[4] = cfg.db_name.clone();
        form.values[5] = cfg.password.clone().unwrap_or_default();
        form
    }

    /// Builds a pre-filled form for editing an existing saved connection.
    pub fn from_connect_config(cfg: &ConnectConfig, editing_index: Option<usize>) -> Self {
        match cfg {
            ConnectConfig::Postgres(cfg) => Self::from_postgres_config(cfg, editing_index),
            ConnectConfig::MySql(cfg) => {
                let mut form = Self {
                    driver: DriverKind::MySql,
                    editing_index,
                    ..Default::default()
                };
                form.values[0] = cfg.name.clone().unwrap_or_default();
                form.values[1] = cfg.host.clone();
                form.values[2] = cfg.port.to_string();
                form.values[3] = cfg.user.clone();
                form.values[4] = cfg.db_name.clone();
                form.values[5] = cfg.password.clone().unwrap_or_default();
                form
            }
        }
    }

    /// Returns true when the form is editing an existing connection.
    pub fn is_editing(&self) -> bool {
        self.editing_index.is_some()
    }

    /// Validates the form and returns a driver-specific connection config.
    pub fn to_connect_config(&self) -> Result<ConnectConfig, String> {
        let parsed = self.parse_values()?;
        match self.driver {
            DriverKind::Postgres => Ok(ConnectConfig::Postgres(PostgresConfig {
                name: parsed.name,
                host: parsed.host,
                user: parsed.user,
                db_name: parsed.db_name,
                port: parsed.port,
                password: parsed.password,
            })),
            DriverKind::MySql => Ok(ConnectConfig::MySql(MySqlConfig {
                name: parsed.name,
                host: parsed.host,
                user: parsed.user,
                db_name: parsed.db_name,
                port: parsed.port,
                password: parsed.password,
            })),
        }
    }

    fn parse_values(&self) -> Result<ParsedFormValues, String> {
        let name = self.values[0].trim().to_string();
        let host = self.values[1].trim().to_string();
        let port_str = self.values[2].trim().to_string();
        let user = self.values[3].trim().to_string();
        let db_name = self.values[4].trim().to_string();
        let password = self.values[5].trim().to_string();

        if host.is_empty() {
            return Err("Host is required".into());
        }
        if user.is_empty() {
            return Err("User is required".into());
        }
        if db_name.is_empty() {
            return Err("Database is required".into());
        }

        let port: u16 = if port_str.is_empty() {
            self.driver.default_port()
        } else {
            port_str
                .parse()
                .map_err(|_| "Port must be a number 1–65535".to_string())?
        };

        Ok(ParsedFormValues {
            name: if name.is_empty() { None } else { Some(name) },
            host,
            user,
            db_name,
            port,
            password: if password.is_empty() {
                None
            } else {
                Some(password)
            },
        })
    }
}

struct ParsedFormValues {
    name: Option<String>,
    host: String,
    user: String,
    db_name: String,
    port: u16,
    password: Option<String>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::{ConnectConfig, MySqlConfig};
    use crate::state::connection::DriverKind;

    fn filled_form(
        name: &str,
        host: &str,
        port: &str,
        user: &str,
        db: &str,
        pw: &str,
    ) -> FormState {
        let mut f = FormState::default();
        f.values[0] = name.to_string();
        f.values[1] = host.to_string();
        f.values[2] = port.to_string();
        f.values[3] = user.to_string();
        f.values[4] = db.to_string();
        f.values[5] = pw.to_string();
        f
    }

    fn postgres_config(form: &FormState) -> PostgresConfig {
        let ConnectConfig::Postgres(cfg) = form.to_connect_config().unwrap() else {
            panic!("Expected Postgres variant");
        };
        cfg
    }

    #[test]
    fn valid_input_returns_config() {
        let f = filled_form("local", "localhost", "5432", "alice", "mydb", "secret");
        let cfg = postgres_config(&f);
        assert_eq!(cfg.name, Some("local".to_string()));
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.user, "alice");
        assert_eq!(cfg.db_name, "mydb");
        assert_eq!(cfg.password, Some("secret".to_string()));
    }

    #[test]
    fn empty_port_defaults_to_5432() {
        let f = filled_form("", "localhost", "", "alice", "mydb", "");
        let cfg = postgres_config(&f);
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.name, None);
    }

    #[test]
    fn empty_password_gives_none() {
        let f = filled_form("", "localhost", "5432", "alice", "mydb", "");
        let cfg = postgres_config(&f);
        assert_eq!(cfg.password, None);
    }

    #[test]
    fn empty_host_returns_error() {
        let f = filled_form("", "", "5432", "alice", "mydb", "");
        assert!(f.to_connect_config().is_err());
    }

    #[test]
    fn empty_user_returns_error() {
        let f = filled_form("", "localhost", "5432", "", "mydb", "");
        assert!(f.to_connect_config().is_err());
    }

    #[test]
    fn empty_db_returns_error() {
        let f = filled_form("", "localhost", "5432", "alice", "", "");
        assert!(f.to_connect_config().is_err());
    }

    #[test]
    fn invalid_port_returns_error() {
        let f = filled_form("", "localhost", "notaport", "alice", "mydb", "");
        assert!(f.to_connect_config().is_err());
    }

    #[test]
    fn next_field_wraps() {
        let mut f = FormState::default();
        f.focused = 5;
        f.next_field();
        assert_eq!(f.focused, 0);
    }

    #[test]
    fn prev_field_wraps() {
        let mut f = FormState::default();
        f.prev_field();
        assert_eq!(f.focused, FIELD_COUNT - 1);
    }

    #[test]
    fn reset_clears_all() {
        let mut f = filled_form("n", "h", "5432", "u", "d", "pw");
        f.focused = 3;
        f.error = Some("err".to_string());
        f.reset();
        assert_eq!(f.focused, 0);
        assert!(f.error.is_none());
        assert!(f.values[0].is_empty());
    }

    #[test]
    fn from_postgres_config_prefills_edit_form() {
        let cfg = PostgresConfig {
            name: Some("prod".into()),
            host: "db.prod".into(),
            user: "app".into(),
            db_name: "main".into(),
            port: 5433,
            password: Some("secret".into()),
        };

        let form = FormState::from_postgres_config(&cfg, Some(2));

        assert_eq!(form.editing_index, Some(2));
        assert_eq!(form.values[0], "prod");
        assert_eq!(form.values[1], "db.prod");
        assert_eq!(form.values[2], "5433");
        assert_eq!(form.values[3], "app");
        assert_eq!(form.values[4], "main");
        assert_eq!(form.values[5], "secret");
    }

    #[test]
    fn new_for_driver_sets_postgres_defaults() {
        let form = FormState::new_for_driver(DriverKind::Postgres);

        assert_eq!(form.driver, DriverKind::Postgres);
        assert_eq!(form.values[2], "5432");
    }

    #[test]
    fn new_for_driver_sets_mysql_defaults() {
        let form = FormState::new_for_driver(DriverKind::MySql);

        assert_eq!(form.driver, DriverKind::MySql);
        assert_eq!(form.values[2], "3306");
    }

    #[test]
    fn valid_input_returns_connect_config_for_selected_driver() {
        let mut form = filled_form("shop", "mysql.local", "", "root", "shop", "secret");
        form.driver = DriverKind::MySql;

        let cfg = form.to_connect_config().unwrap();

        let ConnectConfig::MySql(cfg) = cfg else {
            panic!("Expected MySql variant");
        };
        assert_eq!(cfg.name, Some("shop".to_string()));
        assert_eq!(cfg.host, "mysql.local");
        assert_eq!(cfg.user, "root");
        assert_eq!(cfg.db_name, "shop");
        assert_eq!(cfg.port, 3306);
        assert_eq!(cfg.password, Some("secret".to_string()));
    }

    #[test]
    fn from_connect_config_prefills_mysql_edit_form() {
        let cfg = ConnectConfig::MySql(MySqlConfig {
            name: Some("shop".into()),
            host: "mysql.local".into(),
            user: "root".into(),
            db_name: "shop".into(),
            port: 3307,
            password: None,
        });

        let form = FormState::from_connect_config(&cfg, Some(1));

        assert_eq!(form.driver, DriverKind::MySql);
        assert_eq!(form.editing_index, Some(1));
        assert_eq!(form.values[0], "shop");
        assert_eq!(form.values[1], "mysql.local");
        assert_eq!(form.values[2], "3307");
        assert_eq!(form.values[3], "root");
        assert_eq!(form.values[4], "shop");
        assert_eq!(form.values[5], "");
    }
}
