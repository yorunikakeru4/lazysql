use crate::db::postgres::init::PostgresRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::tables_repo::{
    ColumnInfo, Database, FetchRowsResult, RowData, SqlExecuteOptions, SqlExecuteResult, SqlPage,
    Table, TableField, TableRef, parse_constraint,
};
use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;

/// Detects if SQL returns rows (SELECT-like) using sqlparser.
// TODO: this is a heuristic to route to query vs execute; it may misclassify some statements (e.g. DDL with RETURNING), but should be correct for common cases and let DB report errors for edge cases.
pub fn is_returning_query(sql: &str) -> bool {
    let dialect = PostgreSqlDialect {};
    let Ok(statements) = Parser::parse_sql(&dialect, sql) else {
        return false; // parse error → treat as command, let DB report error
    };
    let Some(stmt) = statements.first() else {
        return false;
    };
    matches!(stmt, Statement::Query(_) | Statement::Explain { .. })
}

impl PostgresRepo {
    async fn fetch_query_rows(
        &self,
        sql: &str,
        page: Option<SqlPage>,
    ) -> Result<FetchRowsResult, DbError> {
        let Some(page) = page else {
            let stmt = self.client.prepare(sql).await.map_err(DbError::Postgres)?;
            let columns = columns_from_statement(&stmt);
            let rows = self
                .client
                .query(&stmt, &[])
                .await
                .map_err(DbError::Postgres)?;
            let data = rows_to_data(&rows);

            return Ok(FetchRowsResult {
                columns,
                total_count: data.len() as u64,
                rows: data,
            });
        };

        let count_sql = format!("SELECT COUNT(*) FROM ({}) AS _subq", sql);
        let count_row = self
            .client
            .query_one(&count_sql, &[])
            .await
            .map_err(DbError::Postgres)?;
        let total_count: i64 = count_row.get(0);

        let data_sql = format!(
            "SELECT * FROM ({}) AS _subq LIMIT {} OFFSET {}",
            sql, page.limit, page.offset
        );

        let stmt = self
            .client
            .prepare(&data_sql)
            .await
            .map_err(DbError::Postgres)?;
        let columns = columns_from_statement(&stmt);
        let rows = self
            .client
            .query(&stmt, &[])
            .await
            .map_err(DbError::Postgres)?;

        Ok(FetchRowsResult {
            columns,
            rows: rows_to_data(&rows),
            total_count: total_count as u64,
        })
    }
}

impl Database for PostgresRepo {
    // TODO: добавить помимо связуюшей таблицы, ещё и связующее поле (для определения связей между таблицами, например, для генерации ER диаграммы)
    async fn get_tables(&self, table_names: Vec<String>) -> Result<Vec<Table>, DbError> {
        let rows = self
            .client
            .query(
                "WITH tables_ AS (
                    SELECT unnest($1::text[]) AS table_name
                ),
                columns_ AS (
                    SELECT
                    isc.table_name,
                    isc.column_name,
                    isc.data_type,
                    isc.is_nullable,
                    isc.column_default AS default
                    FROM tables_ t
                    JOIN information_schema.columns isc ON isc.table_name = t.table_name
                ),
                constraints_ AS (
                    SELECT
                    kcu.table_name,
                    kcu.column_name,
                    tc.constraint_type,
                    c2.table_name AS referenced_table
                    FROM information_schema.key_column_usage kcu
                    JOIN information_schema.table_constraints tc
                    ON tc.constraint_name = kcu.constraint_name
                    LEFT JOIN information_schema.referential_constraints rc
                    ON rc.constraint_name = tc.constraint_name
                    LEFT JOIN information_schema.table_constraints c2
                    ON rc.unique_constraint_name = c2.constraint_name
                )
                    SELECT
                    c.table_name,
                    c.column_name,
                    c.data_type,
                    c.is_nullable,
                    cons.constraint_type AS constraint,
                    cons.referenced_table,
                    c.default
                        FROM columns_ c
                        LEFT JOIN constraints_ cons
                        ON cons.table_name = c.table_name AND cons.column_name = c.column_name
                        ORDER BY c.table_name, c.column_name;",
                &[&table_names],
            )
            .await
            .map_err(DbError::Postgres)
            .and_then(|rows| {
                if rows.is_empty() {
                    Err(DbError::NotFound("Tables not found".to_string()))
                } else {
                    Ok(rows)
                }
            })?;

        let mut tables_info: HashMap<String, Vec<TableField>> = HashMap::new();
        for el in &rows {
            let table_name: String = el.get(0);
            let field = TableField {
                name: el.get(1),
                data_type: el.get(2),
                is_nullable: el.get(3),
                constraint: parse_constraint(el.get(4), el.get(5)),
                default_value: el.get(6),
            };
            tables_info.entry(table_name).or_default().push(field);
        }

        let tables = tables_info
            .into_iter()
            .map(|(name, fields)| Table { name, fields })
            .collect::<Vec<_>>();

        Ok(tables)
    }

    async fn get_schemas(&self) -> Result<Vec<TableRef>, DbError> {
        let rows = self
            .client
            .query(
                "
            SELECT table_schema, table_name
            FROM information_schema.tables
            WHERE table_type='BASE TABLE'
              AND table_schema NOT IN ('pg_catalog', 'information_schema');
            ",
                &[],
            )
            .await
            .map_err(DbError::Postgres)?;
        let schemas = rows
            .into_iter()
            .map(|x| TableRef {
                schema: x.get(0),
                name: x.get(1),
            })
            .collect();

        Ok(schemas)
    }

    async fn execute_sql_with_options(
        &self,
        sql: &str,
        options: Option<SqlExecuteOptions>,
    ) -> Result<SqlExecuteResult, DbError> {
        if is_returning_query(sql) {
            let result = self
                .fetch_query_rows(sql, options.and_then(|opts| opts.page))
                .await?;
            return Ok(SqlExecuteResult::RowsReturned(result));
        }

        let n = self
            .client
            .execute(sql, &[])
            .await
            .map_err(DbError::Postgres)?;
        Ok(SqlExecuteResult::RowsAffected(n))
    }

    async fn fetch_rows(
        &self,
        schema: &str,
        table: &str,
        limit: u16,
        offset: u64,
    ) -> Result<FetchRowsResult, DbError> {
        let esc_schema = schema.replace('"', "\"\"");
        let esc_table = table.replace('"', "\"\"");

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM \"{esc_schema}\".\"{esc_table}\"");
        let count_row = self
            .client
            .query_one(&count_sql, &[])
            .await
            .map_err(DbError::Postgres)?;
        let total_count: i64 = count_row.get(0);

        // Probe LIMIT 0: get column metadata and detect id column
        let probe_sql = format!("SELECT * FROM \"{esc_schema}\".\"{esc_table}\" LIMIT 0");
        let probe = self
            .client
            .prepare(&probe_sql)
            .await
            .map_err(DbError::Postgres)?;
        let has_id = probe.columns().iter().any(|c| c.name() == "id");
        let columns = columns_from_statement(&probe);

        let order = if has_id { " ORDER BY \"id\"" } else { "" };
        let data_sql = format!(
            "SELECT * FROM \"{esc_schema}\".\"{esc_table}\"{order} LIMIT {limit} OFFSET {offset}"
        );

        let rows = self
            .client
            .query(&data_sql, &[])
            .await
            .map_err(DbError::Postgres)?;

        let data = rows_to_data(&rows);

        Ok(FetchRowsResult {
            columns,
            rows: data,
            total_count: total_count as u64,
        })
    }
}

fn columns_from_statement(stmt: &tokio_postgres::Statement) -> Vec<ColumnInfo> {
    stmt.columns()
        .iter()
        .map(|c| ColumnInfo {
            name: c.name().to_string(),
        })
        .collect()
}

fn rows_to_data(rows: &[tokio_postgres::Row]) -> Vec<RowData> {
    rows.iter()
        .map(|row| {
            (0..row.len())
                .map(|i| row_value_to_string(row, i))
                .collect()
        })
        .collect()
}

/// Converts a row value at index to Option<String>.
fn row_value_to_string(row: &tokio_postgres::Row, idx: usize) -> Option<String> {
    use tokio_postgres::types::Type;

    let col_type = row.columns()[idx].type_();

    match *col_type {
        Type::BOOL => row.get::<_, Option<bool>>(idx).map(|v| v.to_string()),
        Type::INT2 => row.get::<_, Option<i16>>(idx).map(|v| v.to_string()),
        Type::INT4 => row.get::<_, Option<i32>>(idx).map(|v| v.to_string()),
        Type::INT8 => row.get::<_, Option<i64>>(idx).map(|v| v.to_string()),
        Type::FLOAT4 => row.get::<_, Option<f32>>(idx).map(|v| v.to_string()),
        Type::FLOAT8 => row.get::<_, Option<f64>>(idx).map(|v| v.to_string()),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => row.get::<_, Option<String>>(idx),
        Type::JSON | Type::JSONB => row
            .get::<_, Option<serde_json::Value>>(idx)
            .map(|v| v.to_string()),
        Type::TIMESTAMP => row
            .get::<_, Option<chrono::NaiveDateTime>>(idx)
            .map(|v| v.to_string()),
        Type::TIMESTAMPTZ => row
            .get::<_, Option<chrono::DateTime<chrono::Utc>>>(idx)
            .map(|v| v.to_string()),
        Type::DATE => row
            .get::<_, Option<chrono::NaiveDate>>(idx)
            .map(|v| v.to_string()),
        Type::TIME => row
            .get::<_, Option<chrono::NaiveTime>>(idx)
            .map(|v| v.to_string()),
        Type::BYTEA => row
            .get::<_, Option<Vec<u8>>>(idx)
            .map(|b| b.iter().map(|byte| format!("{byte:02x}")).collect()),
        Type::OID => row.get::<_, Option<u32>>(idx).map(|v| v.to_string()),
        // For unknown types: try text decode; NULL → None, unsupported non-NULL → type placeholder
        _ => row
            .try_get::<_, Option<String>>(idx)
            .unwrap_or_else(|_| Some(format!("<{}>", col_type.name()))),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_config() -> crate::config::PostgresConfig {
        crate::config::PostgresConfig {
            name: None,
            host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            user: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "test_user".to_string()),
            db_name: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "db_test".to_string()),
            port: std::env::var("TEST_DB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            password: std::env::var("TEST_DB_PASSWORD").ok(),
        }
    }

    #[tokio::test]
    async fn get_schemas() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let users: TableRef = client
            .get_schemas()
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.name == "users")
            .unwrap();
        assert_eq!(users.name, "users");
        assert_eq!(users.schema, "public");
    }

    #[tokio::test]
    async fn get_tables() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let table_names = vec!["users".to_string(), "posts".to_string()];
        verify_tables(client, table_names).await;
    }

    async fn verify_tables(client: PostgresRepo, table_names: Vec<String>) {
        let tables = client.get_tables(table_names).await.unwrap();
        let posts = tables.iter().find(|f| f.name == "posts").unwrap();
        let user_id = posts.fields.iter().find(|f| f.name == "user_id").unwrap();
        assert_eq!(user_id.data_type, "integer");
        assert_eq!(user_id.is_nullable, "NO");
        assert_eq!(
            user_id.constraint,
            Some(crate::db::repo::tables_repo::ConstraintType::ForeignKey(
                "users".to_string()
            ))
        );
    }

    #[tokio::test]
    async fn full_db_get() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let schemas = client.get_schemas().await.unwrap();
        let table_names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
        verify_tables(client, table_names).await;
    }

    #[test]
    fn is_returning_query_detects_select() {
        assert!(is_returning_query("SELECT 1"));
        assert!(is_returning_query("SELECT * FROM users"));
        assert!(is_returning_query("select id from posts where id = 1"));
    }

    #[test]
    fn is_returning_query_detects_with_select() {
        assert!(is_returning_query(
            "WITH cte AS (SELECT 1) SELECT * FROM cte"
        ));
    }

    #[test]
    fn is_returning_query_detects_explain() {
        assert!(is_returning_query("EXPLAIN SELECT 1"));
        assert!(is_returning_query("EXPLAIN ANALYZE SELECT * FROM users"));
    }

    #[test]
    fn is_returning_query_rejects_dml() {
        assert!(!is_returning_query("INSERT INTO users (name) VALUES ('x')"));
        assert!(!is_returning_query("UPDATE users SET name = 'y'"));
        assert!(!is_returning_query("DELETE FROM users WHERE id = 1"));
    }

    #[test]
    fn is_returning_query_rejects_ddl() {
        assert!(!is_returning_query("CREATE TABLE foo (id int)"));
        assert!(!is_returning_query("DROP TABLE foo"));
        assert!(!is_returning_query("ALTER TABLE foo ADD COLUMN bar int"));
    }

    #[test]
    fn is_returning_query_handles_invalid_sql() {
        // Invalid SQL should return false (let DB report error)
        assert!(!is_returning_query("NOT VALID SQL AT ALL"));
        assert!(!is_returning_query(""));
    }

    #[tokio::test]
    async fn execute_sql_select_returns_rows() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let result = client.execute_sql("SELECT 1 AS num").await.unwrap();
        match result {
            SqlExecuteResult::RowsReturned(result) => assert_eq!(result.total_count, 1),
            _ => panic!("Expected RowsReturned"),
        }
    }

    #[tokio::test]
    async fn execute_sql_select_multiple_rows() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        // Use generate_series to guarantee multiple rows
        let result = client
            .execute_sql("SELECT generate_series(1, 5)")
            .await
            .unwrap();
        match result {
            SqlExecuteResult::RowsReturned(result) => assert_eq!(result.total_count, 5),
            _ => panic!("Expected RowsReturned"),
        }
    }

    #[tokio::test]
    async fn execute_sql_update_returns_affected() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        // Update with impossible condition — 0 rows affected
        let result = client
            .execute_sql("UPDATE users SET username = username WHERE 1 = 0")
            .await
            .unwrap();
        match result {
            SqlExecuteResult::RowsAffected(n) => assert_eq!(n, 0),
            _ => panic!("Expected RowsAffected"),
        }
    }

    #[tokio::test]
    async fn execute_sql_invalid_returns_error() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let result = client
            .execute_sql("SELECT * FROM nonexistent_table_xyz")
            .await;
        assert!(result.is_err());
    }

    /// Reproduces panic caused by JSONB/TIMESTAMPTZ columns (e.g. achievements table).
    #[tokio::test]
    async fn fetch_rows_handles_jsonb_and_timestamp_columns() {
        let client = PostgresRepo::new(test_config()).await.unwrap();

        client
            .client
            .batch_execute(
                "DROP TABLE IF EXISTS test_complex_types;
                 CREATE TABLE test_complex_types (
                     id         SERIAL PRIMARY KEY,
                     user_id    BIGINT,
                     project_id BIGINT NOT NULL,
                     name       TEXT NOT NULL,
                     data       JSONB NOT NULL DEFAULT '{}',
                     uncompleted BOOLEAN NOT NULL DEFAULT false,
                     updated_at TIMESTAMPTZ
                 );
                 INSERT INTO test_complex_types (project_id, name, data, uncompleted)
                 VALUES (1, 'configure_logs',    '{}', false),
                        (1, 'configure_metrics', '{}', false);",
            )
            .await
            .unwrap();

        let result = client
            .fetch_rows("public", "test_complex_types", 10, 0)
            .await;

        client
            .client
            .execute("DROP TABLE IF EXISTS test_complex_types", &[])
            .await
            .unwrap();

        let fetched = result.expect("fetch_rows must not panic on JSONB/TIMESTAMPTZ columns");
        assert_eq!(fetched.rows.len(), 2);

        let data_idx = fetched
            .columns
            .iter()
            .position(|c| c.name == "data")
            .unwrap();
        assert!(
            fetched
                .rows
                .iter()
                .all(|row| row[data_idx] == Some("{}".to_string())),
            "JSONB column should render as JSON string"
        );

        let updated_idx = fetched
            .columns
            .iter()
            .position(|c| c.name == "updated_at")
            .unwrap();
        assert!(
            fetched.rows.iter().all(|row| row[updated_idx].is_none()),
            "NULL TIMESTAMPTZ should be None"
        );
    }

    #[tokio::test]
    async fn fetch_rows_returns_paginated_data() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let result = client.fetch_rows("public", "users", 10, 0).await.unwrap();
        assert!(!result.columns.is_empty());
    }

    #[tokio::test]
    async fn fetch_rows_respects_limit_offset() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let page1 = client.fetch_rows("public", "users", 1, 0).await.unwrap();
        let page2 = client.fetch_rows("public", "users", 1, 1).await.unwrap();
        // If there's more than 1 row, pages should differ
        if page1.total_count > 1 {
            assert_ne!(page1.rows, page2.rows);
        }
    }

    #[tokio::test]
    async fn execute_sql_with_page_returns_select_results() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let result = client
            .execute_sql_with_options(
                "SELECT 1 AS num, 'test' AS str",
                Some(SqlExecuteOptions {
                    page: Some(SqlPage {
                        limit: 10,
                        offset: 0,
                    }),
                }),
            )
            .await
            .unwrap();
        let SqlExecuteResult::RowsReturned(result) = result else {
            panic!("Expected RowsReturned");
        };
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], Some("1".into()));
        assert_eq!(result.rows[0][1], Some("test".into()));
    }

    #[tokio::test]
    async fn execute_sql_with_page_handles_pagination() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let result = client
            .execute_sql_with_options(
                "SELECT generate_series(1, 10) AS n",
                Some(SqlExecuteOptions {
                    page: Some(SqlPage {
                        limit: 3,
                        offset: 0,
                    }),
                }),
            )
            .await
            .unwrap();
        let SqlExecuteResult::RowsReturned(result) = result else {
            panic!("Expected RowsReturned");
        };
        assert_eq!(result.total_count, 10);
        assert_eq!(result.rows.len(), 3);
    }
}
