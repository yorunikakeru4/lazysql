use crate::config::ConnectConfig;
use crate::db::repo::db_repo::{DbClient, DbError};
use crate::db::repo::sql_repo::{
    Database, FetchRowsResult, SqlExecuteOptions, SqlExecuteResult, SqlPage, Table, TableDetails,
    TableRef,
};
use crate::state::connection::{ActivePane, ConnectState, ConnectionStatus, FormState};
use crate::state::mode::AppMode;
use crate::state::records::{RecordsSource, RecordsState};
use crate::state::search::SearchState;
use crate::state::sql_input::{SqlInputState, SqlResult};
use std::collections::BTreeSet;
use std::time::Duration;

const CONNECTION_STATUS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct AppState {
    pub connections_config: Vec<ConnectConfig>,
    pub connection_statuses: Vec<ConnectionStatus>,
    pub current_db: Option<DbClient>,

    pub mode: AppMode,

    pub table_selected: usize,
    pub schema_selected: usize,
    pub schemas_raw: Vec<TableRef>,
    pub table_details: Option<TableDetails>,
    pub active_pane: ActivePane,
    pub loaded_table: Option<Table>,

    /// States for various UI components. Kept here to persist across screen changes.
    pub connect: ConnectState,
    pub form: FormState,
    pub search: SearchState,
    pub sql_input: SqlInputState,
    pub records: RecordsState,
}

impl AppState {
    pub fn new(connections: Vec<ConnectConfig>) -> Self {
        AppState {
            connection_statuses: vec![ConnectionStatus::Unknown; connections.len()],
            connections_config: connections,
            current_db: None,
            connect: ConnectState::default(),
            form: FormState::default(),
            search: SearchState::default(),
            sql_input: SqlInputState::default(),
            records: RecordsState::default(),
            schemas_raw: Vec::new(),
            schema_selected: 0,
            table_selected: 0,
            loaded_table: None,
            mode: AppMode::default(),
            active_pane: ActivePane::default(),
            table_details: None,
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
        self.schemas_raw
            .iter()
            .filter(|s| s.schema == schema)
            .map(|s| s.name.clone())
            .collect()
    }

    /// Keeps the status list aligned with the connection list.
    pub fn sync_connection_statuses(&mut self) {
        self.connection_statuses
            .resize(self.connections_config.len(), ConnectionStatus::Unknown);
    }

    /// Removes one saved connection and its status entry by index.
    pub fn remove_connection_at(&mut self, index: usize) {
        if index >= self.connections_config.len() {
            return;
        }
        self.connections_config.remove(index);
        if index < self.connection_statuses.len() {
            self.connection_statuses.remove(index);
        }
        self.sync_connection_statuses();
    }

    /// Updates the status for one connection index.
    pub fn set_connection_status(&mut self, index: usize, status: ConnectionStatus) {
        self.sync_connection_statuses();
        let Some(slot) = self.connection_statuses.get_mut(index) else {
            return;
        };
        *slot = status;
    }

    /// Returns the last known status for one connection index.
    pub fn connection_status(&self, index: usize) -> ConnectionStatus {
        self.connection_statuses
            .get(index)
            .copied()
            .unwrap_or(ConnectionStatus::Unknown)
    }

    /// Refreshes the reachability status for all saved connections.
    pub async fn refresh_connection_statuses(&mut self) {
        for index in 0..self.connections_config.len() {
            self.refresh_connection_status(index).await;
        }
    }
    /// Refreshes the reachability status for one saved connection.
    pub async fn refresh_connection_status(&mut self, index: usize) {
        let Some(connect) = self.connections_config.get(index).cloned() else {
            return;
        };
        let status =
            match tokio::time::timeout(CONNECTION_STATUS_TIMEOUT, DbClient::new(connect)).await {
                Ok(Ok(_)) => ConnectionStatus::Online,
                Ok(Err(_)) | Err(_) => ConnectionStatus::Offline,
            };
        self.set_connection_status(index, status);
    }

    /// Tests the unsaved connection form draft and stores its reachability status.
    pub async fn test_form_connection(&mut self) {
        match self.form.to_postgres_config() {
            Ok(cfg) => {
                self.form.error = None;
                let connect = ConnectConfig::Postgres(cfg);
                self.connect.draft_status = Some(Self::connection_status_for_config(connect).await);
            }
            Err(msg) => {
                self.form.error = Some(msg);
                self.connect.draft_status = None;
            }
        }
    }

    async fn connection_status_for_config(connect: ConnectConfig) -> ConnectionStatus {
        match tokio::time::timeout(CONNECTION_STATUS_TIMEOUT, DbClient::new(connect)).await {
            Ok(Ok(_)) => ConnectionStatus::Online,
            Ok(Err(_)) | Err(_) => ConnectionStatus::Offline,
        }
    }

    /// Connection indices filtered by display name.
    pub fn filtered_connection_indices(&self) -> Vec<usize> {
        self.connections_config
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let meta = crate::state::connection::ConnectionMeta::from(c);
                self.search.matches(&meta.name).then_some(i)
            })
            .collect()
    }

    /// Position of the selected connection inside the filtered connection list.
    pub fn selected_filtered_connection_position(&self) -> Option<usize> {
        self.filtered_connection_indices()
            .iter()
            .position(|i| *i == self.connect.selected)
    }

    /// Clamps `connect.selected` to a visible connection after search changes.
    pub fn clamp_connection_selection(&mut self) {
        let indices = self.filtered_connection_indices();
        if indices.is_empty() {
            self.connect.selected = 0;
            return;
        }
        if indices.contains(&self.connect.selected) {
            return;
        }
        self.connect.selected = indices[0];
    }

    /// Moves the connection selection down within the filtered connection list.
    pub fn select_next_filtered_connection(&mut self) {
        let indices = self.filtered_connection_indices();
        if indices.is_empty() {
            return;
        }
        let Some(current) = indices.iter().position(|i| *i == self.connect.selected) else {
            self.connect.selected = indices[0];
            return;
        };
        self.connect.selected = indices[(current + 1) % indices.len()];
    }

    /// Moves the connection selection up within the filtered connection list.
    pub fn select_prev_filtered_connection(&mut self) {
        let indices = self.filtered_connection_indices();
        if indices.is_empty() {
            return;
        }
        let Some(current) = indices.iter().position(|i| *i == self.connect.selected) else {
            self.connect.selected = indices[0];
            return;
        };
        let prev = if current == 0 {
            indices.len() - 1
        } else {
            current - 1
        };
        self.connect.selected = indices[prev];
    }

    /// Selects the first visible connection, if any.
    pub fn select_first_filtered_connection(&mut self) {
        let Some(first) = self.filtered_connection_indices().first().copied() else {
            return;
        };
        self.connect.selected = first;
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

    /// Moves schema selection up within the filtered schema list, wrapping to the end.
    pub fn select_prev_filtered_schema(&mut self) {
        let len = self.filtered_schema_names().len();
        if len == 0 {
            return;
        }
        self.schema_selected = if self.schema_selected == 0 {
            len - 1
        } else {
            self.schema_selected - 1
        };
    }

    /// Moves table selection up within the filtered table list, wrapping to the end.
    pub fn select_prev_filtered_table(&mut self, schema: &str) {
        let len = self.filtered_table_names(schema).len();
        if len == 0 {
            return;
        }
        self.table_selected = if self.table_selected == 0 {
            len - 1
        } else {
            self.table_selected - 1
        };
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

    /// The schema name at the current `schema_selected` index within the filtered list, if any.
    pub fn selected_schema_name(&self) -> Option<String> {
        self.filtered_schema_names()
            .into_iter()
            .nth(self.schema_selected)
    }

    /// Connects to the database at `connect.selected` index.
    pub async fn connect_selected(&mut self) -> Result<(), DbError> {
        self.connect.error = None;
        let idx = self.connect.selected;
        if idx >= self.connections_config.len() {
            let msg = "No connection at selected index".to_string();
            self.connect.error = Some(msg.clone());
            return Err(DbError::NotFound(msg));
        }
        let conn = self.connections_config[idx].clone();
        match tokio::time::timeout(CONNECTION_STATUS_TIMEOUT, DbClient::new(conn)).await {
            Err(_) => {
                let e = DbError::ConnectionTimeout(CONNECTION_STATUS_TIMEOUT);
                self.connect.error = Some(e.to_string());
                self.set_connection_status(idx, ConnectionStatus::Offline);
                Err(e)
            }
            Ok(Ok(client)) => {
                self.current_db = Some(client);
                self.set_connection_status(idx, ConnectionStatus::Online);
                Ok(())
            }
            Ok(Err(e)) => {
                self.connect.error = Some(e.to_string());
                self.set_connection_status(idx, ConnectionStatus::Offline);
                Err(e)
            }
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

    /// Loads full schema details for the selected table into `table_details`.
    pub async fn load_table_details(&mut self, schema: &str, table: &str) -> Result<(), DbError> {
        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };
        self.table_details = Some(repo.get_table_details(schema, table).await?);
        Ok(())
    }

    /// Executes the SQL query in `sql_input.query` and stores the result.
    pub async fn execute_sql_input(&mut self) {
        let query = self.sql_input.query.trim().to_string();
        if query.is_empty() {
            self.sql_input.reset();
            return;
        }

        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            self.sql_input.result = Some(SqlResult::Error("No active connection".to_string()));
            return;
        };

        match repo.execute_sql(&query).await {
            Ok(SqlExecuteResult::RowsAffected(n)) => {
                self.sql_input.result = Some(SqlResult::Success { rows_affected: n });
            }
            Ok(SqlExecuteResult::RowsReturned(result)) => {
                self.sql_input.result = Some(SqlResult::Rows {
                    count: result.total_count as usize,
                });
            }
            Err(e) => {
                self.sql_input.result = Some(SqlResult::Error(format_sql_error(&e)));
            }
        }
    }

    /// Loads records for the currently selected table.
    /// `terminal_height` is used to pick the right `rows_per_page`.
    pub async fn load_table_records(
        &mut self,
        terminal_height: u16,
        terminal_width: u16,
    ) -> Result<(), DbError> {
        let Some(table) = &self.loaded_table else {
            return Err(DbError::NotFound("No table loaded".to_string()));
        };
        let Some(schema) = self.selected_schema_name() else {
            return Err(DbError::NotFound("No schema selected".to_string()));
        };
        let table_name = table.name.clone();

        // height - borders(2) - header(1) - separator(1) - status_bar(3)
        let table_rpp = terminal_height.saturating_sub(7).max(1);

        self.records = RecordsState::for_table(schema.clone(), table_name.clone());
        self.records.rows_per_page = table_rpp;

        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };

        let result = repo.fetch_rows(&schema, &table_name, table_rpp, 0).await?;
        self.records.update_from_result(result);

        let actual_rows_per_page = self
            .records
            .rows_per_page_for_terminal(terminal_height, terminal_width);
        if actual_rows_per_page != self.records.rows_per_page {
            self.records.rows_per_page = actual_rows_per_page;
            let result = repo
                .fetch_rows(&schema, &table_name, actual_rows_per_page, 0)
                .await?;
            self.records.update_from_result(result);
        }

        Ok(())
    }

    /// Fetches the current page of records based on offset.
    pub async fn fetch_records_page(&mut self) -> Result<(), DbError> {
        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };

        let Some(source) = &self.records.source else {
            return Err(DbError::NotFound("No records source".to_string()));
        };

        let limit = self.records.rows_per_page;
        let offset = self.records.offset;

        let result = match source.clone() {
            RecordsSource::Table { schema, table } => {
                repo.fetch_rows(&schema, &table, limit, offset).await?
            }
            RecordsSource::Query { sql } => {
                execute_sql_rows(repo, &sql, SqlPage { limit, offset }).await?
            }
        };

        self.records.update_from_result(result);
        Ok(())
    }

    /// Moves the record cursor down, auto-advancing to the next page at the row boundary.
    pub async fn move_record_down(&mut self, reset_col: bool) {
        if self.records.selected_row + 1 >= self.records.rows.len() && self.records.has_next_page()
        {
            self.records.next_page();
            if self.fetch_records_page().await.is_err() {
                self.records.prev_page();
            } else {
                self.records.selected_row = 0;
            }
        } else {
            self.records.move_row_down();
        }
        if reset_col {
            self.records.selected_col = 0;
        }
    }

    /// Moves the record cursor up, auto-going to the previous page at the row boundary.
    pub async fn move_record_up(&mut self, reset_col: bool) {
        if self.records.selected_row == 0 && self.records.has_prev_page() {
            self.records.prev_page();
            if self.fetch_records_page().await.is_err() {
                self.records.next_page();
            } else {
                self.records.selected_row = self.records.rows.len().saturating_sub(1);
            }
        } else {
            self.records.move_row_up();
        }
        if reset_col {
            self.records.selected_col = 0;
        }
    }

    /// Executes the SQL from `sql_input` and loads results into records state for viewing.
    pub async fn execute_sql_for_records(
        &mut self,
        terminal_height: u16,
        terminal_width: u16,
    ) -> Result<(), DbError> {
        let query = self.sql_input.query.trim().to_string();
        if query.is_empty() {
            return Err(DbError::NotFound("Empty query".to_string()));
        }

        let table_rpp = terminal_height.saturating_sub(7).max(1);

        self.records = RecordsState::for_query(query.clone());
        self.records.rows_per_page = table_rpp;

        let Some(DbClient::Postgres(repo)) = &self.current_db else {
            return Err(DbError::NotFound("No active connection".to_string()));
        };

        let result = execute_sql_rows(
            repo,
            &query,
            SqlPage {
                limit: table_rpp,
                offset: 0,
            },
        )
        .await?;
        self.records.update_from_result(result);

        let actual_rows_per_page = self
            .records
            .rows_per_page_for_terminal(terminal_height, terminal_width);
        if actual_rows_per_page != self.records.rows_per_page {
            self.records.rows_per_page = actual_rows_per_page;
            let result = execute_sql_rows(
                repo,
                &query,
                SqlPage {
                    limit: actual_rows_per_page,
                    offset: 0,
                },
            )
            .await?;
            self.records.update_from_result(result);
        }

        Ok(())
    }
}

/// Formats database errors for the SQL result popup.
pub(crate) fn format_sql_error(error: &DbError) -> String {
    match error {
        DbError::Postgres(error) => {
            let Some(db_error) = error.as_db_error() else {
                return format!("SQL error: {error}");
            };
            let message = db_error.message();
            let mut lines = vec![format!("SQL error: {message}")];
            if let Some(position) = db_error.position() {
                lines.push(format!("Position: {position:?}"));
            }
            if let Some(detail) = db_error.detail() {
                lines.push(format!("Detail: {detail}"));
            }
            if let Some(hint) = db_error.hint() {
                lines.push(format!("Hint: {hint}"));
            }
            let sqlstate = db_error.code().code();
            lines.push(format!("SQLSTATE: {sqlstate}"));
            lines.join("\n")
        }
        DbError::MySql(error) => {
            format!("SQL error: {error}")
        }
        DbError::NotFound(message) => format!("SQL error: {message}"),
        DbError::ConnectionTimeout(timeout) => {
            format!(
                "SQL error: connection timed out after {}s",
                timeout.as_secs()
            )
        }
    }
}

async fn execute_sql_rows(
    repo: &crate::db::postgres::init::PostgresRepo,
    query: &str,
    page: SqlPage,
) -> Result<FetchRowsResult, DbError> {
    match repo
        .execute_sql_with_options(query, Some(SqlExecuteOptions { page: Some(page) }))
        .await?
    {
        SqlExecuteResult::RowsReturned(result) => Ok(result),
        SqlExecuteResult::RowsAffected(_) => Err(DbError::NotFound(
            "SQL did not return rows for records view".to_string(),
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::PostgresConfig;

    fn pg_connect() -> ConnectConfig {
        ConnectConfig::Postgres(PostgresConfig {
            name: None,
            host: "h".to_string(),
            user: "u".to_string(),
            db_name: "d".to_string(),
            port: 5432,
            password: None,
        })
    }

    fn named_pg_connect(name: &str) -> ConnectConfig {
        ConnectConfig::Postgres(PostgresConfig {
            name: Some(name.to_string()),
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
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
            TableRef {
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
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
            TableRef {
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

    #[test]
    fn selected_schema_name_respects_filter() {
        // schemas: ["auth", "public"] — sorted alphabetically
        // filter "pub" → filtered = ["public"], index 0 must return "public", not "auth"
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            TableRef {
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
        ];
        state.search.query = "pub".to_string();
        state.schema_selected = 0;
        assert_eq!(state.selected_schema_name(), Some("public".to_string()));
    }

    #[tokio::test]
    async fn connect_selected_out_of_bounds_returns_error() {
        let mut state = AppState::new(vec![]);
        let result = state.connect_selected().await;
        assert!(result.is_err());
        assert!(state.connect.error.is_some());
    }

    #[tokio::test]
    async fn connect_selected_sets_error_on_failure() {
        let mut state = AppState::new(vec![ConnectConfig::Postgres(PostgresConfig {
            name: Some("bad".to_string()),
            host: "127.0.0.1".to_string(),
            user: "postgres".to_string(),
            db_name: "postgres".to_string(),
            port: 1,
            password: Some("wrong".to_string()),
        })]);

        let result = state.connect_selected().await;

        assert!(result.is_err());
        assert!(state.connect.error.is_some());
    }

    #[test]
    fn filtered_schema_names_empty_query_returns_all() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
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
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
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
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
        ];
        state.search.query = "user".to_string();
        let result = state.filtered_table_names("public");
        assert_eq!(result, vec!["users"]);
    }

    #[test]
    fn select_prev_filtered_schema_wraps_to_last_visible_schema() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
                schema: "auth".to_string(),
                name: "tokens".to_string(),
            },
        ];
        state.schema_selected = 0;

        state.select_prev_filtered_schema();

        assert_eq!(state.schema_selected, 1);
    }

    #[test]
    fn select_prev_filtered_table_wraps_to_last_visible_table() {
        let mut state = AppState::new(vec![pg_connect()]);
        state.schemas_raw = vec![
            TableRef {
                schema: "public".to_string(),
                name: "users".to_string(),
            },
            TableRef {
                schema: "public".to_string(),
                name: "posts".to_string(),
            },
        ];
        state.table_selected = 0;

        state.select_prev_filtered_table("public");

        assert_eq!(state.table_selected, 1);
    }

    #[test]
    fn filtered_connection_indices_filters_by_display_name() {
        let mut state = AppState::new(vec![
            named_pg_connect("Local Dev"),
            named_pg_connect("Staging"),
            named_pg_connect("Production"),
        ]);
        state.search.query = "dev".to_string();

        assert_eq!(state.filtered_connection_indices(), vec![0]);
    }

    #[test]
    fn filtered_connection_indices_matches_case_insensitive() {
        let mut state = AppState::new(vec![named_pg_connect("Local Dev")]);
        state.search.query = "LOCAL".to_string();

        assert_eq!(state.filtered_connection_indices(), vec![0]);
    }

    #[test]
    fn clamp_connection_selection_moves_to_first_visible_match() {
        let mut state = AppState::new(vec![
            named_pg_connect("Local Dev"),
            named_pg_connect("Staging"),
            named_pg_connect("Production"),
        ]);
        state.connect.selected = 0;
        state.search.query = "prod".to_string();

        state.clamp_connection_selection();

        assert_eq!(state.connect.selected, 2);
    }

    #[test]
    fn select_prev_filtered_connection_wraps_to_last_visible_match() {
        let mut state = AppState::new(vec![
            named_pg_connect("Local Dev"),
            named_pg_connect("Staging"),
            named_pg_connect("Production"),
        ]);
        state.search.query = "i".to_string();
        state.connect.selected = 1;

        state.select_prev_filtered_connection();

        assert_eq!(state.connect.selected, 2);
    }

    #[test]
    fn sql_error_message_keeps_context_without_driver_noise() {
        let message = format_sql_error(&DbError::NotFound("Empty query".to_string()));

        assert_eq!(message, "SQL error: Empty query");
    }

    #[test]
    fn selected_filtered_connection_position_is_none_without_match() {
        let mut state = AppState::new(vec![named_pg_connect("Local Dev")]);
        state.search.query = "missing".to_string();

        assert_eq!(state.selected_filtered_connection_position(), None);
    }

    #[test]
    fn new_initializes_unknown_connection_statuses() {
        let state = AppState::new(vec![
            named_pg_connect("Local Dev"),
            named_pg_connect("Prod"),
        ]);

        assert_eq!(
            state.connection_statuses,
            vec![ConnectionStatus::Unknown, ConnectionStatus::Unknown]
        );
    }

    #[test]
    fn sync_connection_statuses_tracks_connection_count() {
        let mut state = AppState::new(vec![named_pg_connect("Local Dev")]);
        state.set_connection_status(0, ConnectionStatus::Online);
        state.connections_config.push(named_pg_connect("Prod"));

        state.sync_connection_statuses();

        assert_eq!(
            state.connection_statuses,
            vec![ConnectionStatus::Online, ConnectionStatus::Unknown]
        );

        state.connections_config.pop();
        state.sync_connection_statuses();

        assert_eq!(state.connection_statuses, vec![ConnectionStatus::Online]);
    }

    #[test]
    fn remove_connection_at_keeps_statuses_aligned_for_middle_index() {
        let mut state = AppState::new(vec![
            named_pg_connect("Local Dev"),
            named_pg_connect("Staging"),
            named_pg_connect("Prod"),
        ]);
        state.set_connection_status(0, ConnectionStatus::Online);
        state.set_connection_status(1, ConnectionStatus::Offline);
        state.set_connection_status(2, ConnectionStatus::Unknown);

        state.remove_connection_at(1);

        assert_eq!(state.connections_config.len(), 2);
        assert_eq!(
            state.connection_statuses,
            vec![ConnectionStatus::Online, ConnectionStatus::Unknown]
        );
    }

    #[tokio::test]
    async fn test_form_connection_marks_invalid_draft_offline_without_saving() {
        let mut state = AppState::new(vec![]);
        state.form.values[1] = "127.0.0.1".to_string();
        state.form.values[2] = "1".to_string();
        state.form.values[3] = "postgres".to_string();
        state.form.values[4] = "postgres".to_string();

        state.test_form_connection().await;

        assert!(state.connections_config.is_empty());
        assert_eq!(state.connect.draft_status, Some(ConnectionStatus::Offline));
    }

    #[tokio::test]
    async fn test_form_connection_marks_test_database_online() {
        let mut state = AppState::new(vec![]);
        state.form.values[0] = "test-db".to_string();
        state.form.values[1] =
            std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        state.form.values[2] = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5439".to_string());
        state.form.values[3] =
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "test_user".to_string());
        state.form.values[4] =
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "db_test".to_string());
        state.form.values[5] = std::env::var("TEST_DB_PASSWORD").unwrap_or_default();

        state.test_form_connection().await;

        assert_eq!(state.connect.draft_status, Some(ConnectionStatus::Online));
    }
}
