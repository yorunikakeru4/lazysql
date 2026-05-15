use crate::db::postgres::init::PostgresRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::sql_repo::{
    ColumnInfo, Database, FetchRowsResult, FkRef, IndexInfo, RowData, SqlExecuteOptions,
    SqlExecuteResult, SqlPage, Table, TableDetails, TableField, TableRef, parse_constraint,
};
use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

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

/// Builds (count_sql, data_sql) for pagination.
///
/// CTE queries (`WITH ... SELECT`) cannot be wrapped in a subquery — PostgreSQL
/// forbids `SELECT COUNT(*) FROM (WITH ...) AS s`. Instead we append the query
/// body as an extra CTE and select from that.
fn build_pagination_sqls(sql: &str, limit: u16, offset: u64) -> (String, String) {
    if let Some((with_str, body)) = try_extract_cte(sql) {
        let count_sql =
            format!("{with_str}, _lazysql_count AS ({body}) SELECT COUNT(*) FROM _lazysql_count");
        let data_sql = format!(
            "{with_str}, _lazysql_data AS ({body}) SELECT * FROM _lazysql_data LIMIT {limit} OFFSET {offset}"
        );
        return (count_sql, data_sql);
    }
    (
        format!("SELECT COUNT(*) FROM ({sql}) AS _subq"),
        format!("SELECT * FROM ({sql}) AS _subq LIMIT {limit} OFFSET {offset}"),
    )
}

/// If `sql` is a CTE query, returns `(with_clause_str, body_str)` with the
/// WITH clause separated from the final SELECT so each can be recombined.
fn try_extract_cte(sql: &str) -> Option<(String, String)> {
    let dialect = PostgreSqlDialect {};
    let stmts = Parser::parse_sql(&dialect, sql).ok()?;
    let Statement::Query(mut query) = stmts.into_iter().next()? else {
        return None;
    };
    let with = query.with.take()?;
    Some((format!("{with}"), format!("{query}")))
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

        let (count_sql, data_sql) = build_pagination_sqls(sql, page.limit, page.offset);
        let count_row = self
            .client
            .query_one(&count_sql, &[])
            .await
            .map_err(DbError::Postgres)?;
        let total_count: i64 = count_row.get(0);

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
            total_count: u64::try_from(total_count).unwrap_or(0),
        })
    }
}

impl Database for PostgresRepo {
    // TODO: добавить помимо связуюшей таблицы, ещё и связующее поле (для определения связей между таблицами, например, для генерации ER диаграммы)
    async fn get_tables(&self, table_names: Vec<String>) -> Result<Vec<Table>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT DISTINCT isc.table_name
                 FROM information_schema.columns isc
                 WHERE isc.table_name = ANY($1::text[])
                 ORDER BY isc.table_name;",
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

        let tables = rows
            .into_iter()
            .map(|row| Table { name: row.get(0) })
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
            total_count: u64::try_from(total_count).unwrap_or(0),
        })
    }

    async fn get_table_details(&self, schema: &str, table: &str) -> Result<TableDetails, DbError> {
        // 1. columns
        let col_rows = self
            .client
            .query(
                "SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                tc.constraint_type,
                rc2.table_name AS referenced_table,
                c.column_default
             FROM information_schema.columns c
             LEFT JOIN information_schema.key_column_usage kcu
                 ON kcu.table_schema = c.table_schema
                 AND kcu.table_name  = c.table_name
                 AND kcu.column_name = c.column_name
             LEFT JOIN information_schema.table_constraints tc
                 ON tc.constraint_name = kcu.constraint_name
                 AND tc.table_schema   = c.table_schema
             LEFT JOIN information_schema.referential_constraints rc
                 ON rc.constraint_name = tc.constraint_name
             LEFT JOIN information_schema.table_constraints rc2
                 ON rc2.constraint_name = rc.unique_constraint_name
             WHERE c.table_schema = $1 AND c.table_name = $2
             ORDER BY c.ordinal_position",
                &[&schema, &table],
            )
            .await
            .map_err(DbError::Postgres)?;

        let fields: Vec<TableField> = col_rows
            .iter()
            .map(|r| TableField {
                name: r.get(0),
                data_type: r.get(1),
                is_nullable: r.get(2),
                constraint: parse_constraint(r.get(3), r.get(4)),
                default_value: r.get(5),
            })
            .collect();

        // 2. row count + size
        let stat_row = self
            .client
            .query_opt(
                "SELECT reltuples::bigint, pg_size_pretty(pg_total_relation_size(c.oid))
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND c.relname = $2",
                &[&schema, &table],
            )
            .await
            .map_err(DbError::Postgres)?;
        let (row_count, size_pretty) = stat_row
            .map(|r| (r.get::<_, Option<i64>>(0), r.get::<_, Option<String>>(1)))
            .unwrap_or((None, None));

        // 3. indexes
        let idx_rows = self
            .client
            .query(
                "SELECT indexname, indexdef
             FROM pg_indexes
             WHERE schemaname = $1 AND tablename = $2",
                &[&schema, &table],
            )
            .await
            .map_err(DbError::Postgres)?;

        let indexes: Vec<IndexInfo> = idx_rows
            .iter()
            .map(|r| {
                let name: String = r.get(0);
                IndexInfo { name }
            })
            .collect();

        // 4. inbound FK refs
        let fk_rows = self
            .client
            .query(
                "SELECT src_tc.table_name, src_kcu.column_name
             FROM information_schema.referential_constraints rc
             JOIN information_schema.table_constraints src_tc
                 ON src_tc.constraint_name = rc.constraint_name
             JOIN information_schema.key_column_usage src_kcu
                 ON src_kcu.constraint_name = rc.constraint_name
             JOIN information_schema.table_constraints tgt_tc
                 ON tgt_tc.constraint_name = rc.unique_constraint_name
             WHERE tgt_tc.table_schema = $1 AND tgt_tc.table_name = $2",
                &[&schema, &table],
            )
            .await
            .map_err(DbError::Postgres)?;

        let fk_refs: Vec<FkRef> = fk_rows
            .iter()
            .map(|r| FkRef {
                from_table: r.get(0),
                column: r.get(1),
            })
            .collect();

        Ok(TableDetails {
            name: table.to_string(),
            schema: schema.to_string(),
            row_count,
            size_pretty,
            fields,
            indexes,
            fk_refs,
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
        _ => row.try_get::<_, Option<String>>(idx).unwrap_or_else(|_| {
            let type_name = col_type.name();
            Some(format!("<{type_name}>"))
        }),
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
                .unwrap_or(5439),
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
        assert!(tables.iter().any(|f| f.name == "posts"));
        assert!(tables.iter().any(|f| f.name == "users"));
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
    async fn get_table_details_returns_fields_and_indexes() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let details = client.get_table_details("public", "users").await.unwrap();
        assert_eq!(details.name, "users");
        assert!(!details.fields.is_empty());
        assert!(!details.indexes.is_empty());
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
