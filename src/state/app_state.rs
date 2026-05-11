use crate::config::Connect;
use crate::config::Connect::Postgres;
use crate::db::repo::db_repo::{DbClient, DbError};
use crate::db::repo::tables_repo::{Database, Schema, Table};
use crate::state::connect::ConnectState;
use crate::state::form::FormState;
use crate::state::search::SearchState;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct AppState {
    pub connections: Vec<Connect>,
    pub current_db: Option<DbClient>,
    pub connect: ConnectState,
    pub form: FormState,
    pub search: SearchState,
    pub schemas_raw: Vec<Schema>,
    pub schema_selected: usize,
    pub table_selected: usize,
    pub loaded_table: Option<Table>,
}

impl AppState {
    pub fn new(connections: Vec<Connect>) -> Self {
        AppState {
            connections,
            current_db: None,
            connect: ConnectState::default(),
            form: FormState::default(),
            search: SearchState::default(),
            schemas_raw: Vec::new(),
            schema_selected: 0,
            table_selected: 0,
            loaded_table: None,
        }
    }

    /// Unique schema names sorted alphabetically.
    pub fn schema_names(&self) -> Vec<String> {
        self.schemas_raw
            .iter()
            .map(|s| s.schema.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Table names within the given schema.
    pub fn table_names_in_schema(&self, schema: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .schemas_raw
            .iter()
            .filter(|s| s.schema == schema)
            .map(|s| s.name.clone())
            .collect();
        names.sort();
        names
    }

    /// Schema names filtered by `search.query` (case-insensitive, empty query = all).
    pub fn filtered_schema_names(&self) -> Vec<String> {
        self.schema_names()
            .into_iter()
            .filter(|n| self.search.matches(n))
            .collect()
    }

    /// Table names within `schema` filtered by `search.query`.
    pub fn filtered_table_names(&self, schema: &str) -> Vec<String> {
        self.table_names_in_schema(schema)
            .into_iter()
            .filter(|n| self.search.matches(n))
            .collect()
    }

    /// Clamps `schema_selected` and `table_selected` to the lengths of the
    /// currently filtered lists. Call after every query change.
    pub fn clamp_search_selections(&mut self) {
        let schema_len = self.filtered_schema_names().len();
        if schema_len == 0 {
            self.schema_selected = 0;
        } else {
            self.schema_selected = self.schema_selected.min(schema_len - 1);
        }

        let table_len = self
            .selected_schema_name()
            .map(|s| self.filtered_table_names(&s).len())
            .unwrap_or(0);
        if table_len == 0 {
            self.table_selected = 0;
        } else {
            self.table_selected = self.table_selected.min(table_len - 1);
        }
    }

    /// The schema name at the current `schema_selected` index, if any.
    pub fn selected_schema_name(&self) -> Option<String> {
        self.schema_names().into_iter().nth(self.schema_selected)
    }

    /// Connects to the database at `connect.selected` index.
    pub async fn connect_selected(&mut self) -> Result<(), DbError> {
        self.connect.error = None;
        let idx = self.connect.selected;
        if idx >= self.connections.len() {
            let msg = "No connection at selected index".to_string();
            self.connect.error = Some(msg.clone());
            return Err(DbError::NotFound(msg));
        }
        match &self.connections[idx] {
            Postgres(_) => match DbClient::new(self.connections[idx].clone()).await {
                Ok(client) => {
                    self.current_db = Some(client);
                    Ok(())
                }
                Err(e) => {
                    self.connect.error = Some(e.to_string());
                    Err(e)
                }
            },
        }
    }

    /// Loads all schemas from the active connection into `schemas_raw`.
    pub async fn load_schemas(&mut self) -> Result<(), DbError> {
        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };
        self.schemas_raw = repo.get_schemas().await?;
        self.schema_selected = 0;
        self.table_selected = 0;
        Ok(())
    }

    /// Loads full field details for a table by explicit name (used when the
    /// table was selected from a filtered list).
    pub async fn load_table_by_name(&mut self, name: String) -> Result<(), DbError> {
        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };
        let mut tables = repo.get_tables(vec![name]).await?;
        self.loaded_table = tables.pop();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::PostgresConfig;

    fn pg_connect() -> Connect {
        Connect::Postgres(PostgresConfig {
            host: "h".to_string(),
            user: "u".to_string(),
            db_name: "d".to_string(),
            port: 5432,
            password: None,
        })
    }

    #[test]
    fn schema_names_deduped_and_sorted() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
        ];
        let names = state.schema_names();
        assert_eq!(names, vec!["auth", "public"]);
    }

    #[test]
    fn table_names_for_schema() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
        ];
        let tables = state.table_names_in_schema("public");
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"posts".to_string()));
        assert_eq!(tables.len(), 2);
    }

    #[test]
    fn selected_schema_name_returns_none_when_empty() {
        let state = AppState::new(vec![pg_connect()]);
        assert_eq!(state.selected_schema_name(), None);
    }

    #[tokio::test]
    async fn connect_selected_out_of_bounds_returns_error() {
        let mut state = AppState::new(vec![]);
        let result = state.connect_selected().await;
        assert!(result.is_err());
        assert!(state.connect.error.is_some());
    }

    #[test]
    fn filtered_schema_names_empty_query_returns_all() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
        ];
        assert_eq!(state.filtered_schema_names(), vec!["auth", "public"]);
    }

    #[test]
    fn filtered_schema_names_filters_by_query() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
        ];
        state.search.query = "pub".to_string();
        assert_eq!(state.filtered_schema_names(), vec!["public"]);
    }

    #[test]
    fn filtered_table_names_filters_by_query() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            Schema {
                catalog: "d".to_string(),
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
        ];
        state.search.query = "user".to_string();
        let result = state.filtered_table_names("public");
        assert_eq!(result, vec!["users"]);
    }
}
