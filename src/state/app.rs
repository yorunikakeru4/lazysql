use crate::config::Connect;
use crate::config::Connect::Postgres;
use crate::db::repo::db_repo::{DbClient, DbError};
use crate::db::repo::tables_repo::{
    Database, FetchRowsResult, SqlExecuteOptions, SqlExecuteResult, SqlPage, Table, TableDetails,
    TableRef,
};
use crate::state::connection::{ActivePane, ConnectState, FormState};
use crate::state::mode::AppMode;
use crate::state::records::{RecordsSource, RecordsState};
use crate::state::search::SearchState;
use crate::state::sql_input::{SqlInputState, SqlResult};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct AppState {
    pub connections: Vec<Connect>,
    pub current_db: Option<DbClient>,
    pub connect: ConnectState,
    pub form: FormState,
    pub search: SearchState,
    pub sql_input: SqlInputState,
    pub records: RecordsState,
    pub schemas_raw: Vec<TableRef>,
    pub schema_selected: usize,
    pub table_selected: usize,
    pub loaded_table: Option<Table>,
    pub mode: AppMode,
    pub active_pane: ActivePane,
    pub help_visible: bool,
    pub table_details: Option<TableDetails>,
}

impl AppState {
    pub fn new(connections: Vec<Connect>) -> Self {
        AppState {
            connections,
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
            help_visible: false,
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

    /// Loads full schema details for the selected table into `table_details`.
    #[allow(dead_code)]
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
                self.sql_input.result = Some(SqlResult::Error(e.to_string()));
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
        let schema = self
            .selected_schema_name()
            .unwrap_or_else(|| "public".to_string());
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

    fn pg_connect() -> Connect {
        Connect::Postgres(PostgresConfig {
            name: None,
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
}
