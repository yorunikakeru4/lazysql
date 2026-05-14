use crate::config::PostgresConfig;

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
    /// Moves focus to the next field, wrapping around.
    pub fn next_field(&mut self) {
        self.focused = (self.focused + 1) % FIELD_COUNT;
    }

    /// Moves focus to the previous field, stopping at 0.
    pub fn prev_field(&mut self) {
        self.focused = self.focused.saturating_sub(1);
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

    /// Returns true when the form is editing an existing connection.
    pub fn is_editing(&self) -> bool {
        self.editing_index.is_some()
    }

    /// Validates the form and returns a `PostgresConfig` on success.
    pub fn to_postgres_config(&self) -> Result<PostgresConfig, String> {
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
            5432
        } else {
            port_str
                .parse()
                .map_err(|_| "Port must be a number 1–65535".to_string())?
        };

        Ok(PostgresConfig {
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

#[cfg(test)]
mod test {
    use super::*;

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

    #[test]
    fn valid_input_returns_config() {
        let f = filled_form("local", "localhost", "5432", "alice", "mydb", "secret");
        let cfg = f.to_postgres_config().unwrap();
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
        let cfg = f.to_postgres_config().unwrap();
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.name, None);
    }

    #[test]
    fn empty_password_gives_none() {
        let f = filled_form("", "localhost", "5432", "alice", "mydb", "");
        let cfg = f.to_postgres_config().unwrap();
        assert_eq!(cfg.password, None);
    }

    #[test]
    fn empty_host_returns_error() {
        let f = filled_form("", "", "5432", "alice", "mydb", "");
        assert!(f.to_postgres_config().is_err());
    }

    #[test]
    fn empty_user_returns_error() {
        let f = filled_form("", "localhost", "5432", "", "mydb", "");
        assert!(f.to_postgres_config().is_err());
    }

    #[test]
    fn empty_db_returns_error() {
        let f = filled_form("", "localhost", "5432", "alice", "", "");
        assert!(f.to_postgres_config().is_err());
    }

    #[test]
    fn invalid_port_returns_error() {
        let f = filled_form("", "localhost", "notaport", "alice", "mydb", "");
        assert!(f.to_postgres_config().is_err());
    }

    #[test]
    fn next_field_wraps() {
        let mut f = FormState::default();
        f.focused = 5;
        f.next_field();
        assert_eq!(f.focused, 0);
    }

    #[test]
    fn prev_field_stops_at_zero() {
        let mut f = FormState::default();
        f.prev_field();
        assert_eq!(f.focused, 0);
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
}
