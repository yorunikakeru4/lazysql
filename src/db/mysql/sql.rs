use mysql_async::prelude::*;
use sqlparser::ast::Statement;
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use crate::db::mysql::init::MySqlRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::sql_repo::{
    ColumnInfo, Database, FetchRowsResult, RowData, SqlExecuteOptions, SqlExecuteResult, SqlPage,
    Table, TableDetails, TableRef,
};

/// Detects if SQL is a SELECT-like query using MySQL dialect.
pub fn is_returning_query(sql: &str) -> bool {
    let dialect = MySqlDialect {};
    let Ok(statements) = Parser::parse_sql(&dialect, sql) else {
        return false;
    };
    let Some(stmt) = statements.first() else {
        return false;
    };
    matches!(stmt, Statement::Query(_) | Statement::Explain { .. })
}

/// Builds (count_sql, data_sql) for pagination. Handles CTEs via MySQL dialect.
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

fn try_extract_cte(sql: &str) -> Option<(String, String)> {
    let dialect = MySqlDialect {};
    let stmts = Parser::parse_sql(&dialect, sql).ok()?;
    let Statement::Query(mut query) = stmts.into_iter().next()? else {
        return None;
    };
    let with = query.with.take()?;
    Some((format!("{with}"), format!("{query}")))
}

fn mysql_value_to_string(val: mysql_async::Value) -> Option<String> {
    use mysql_async::Value;
    match val {
        Value::NULL => None,
        Value::Bytes(b) => String::from_utf8(b).ok().or(Some("<binary>".into())),
        Value::Int(i) => Some(i.to_string()),
        Value::UInt(u) => Some(u.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Double(d) => Some(d.to_string()),
        Value::Date(y, mo, d, h, mi, s, _us) => {
            Some(format!("{y}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}"))
        }
        Value::Time(neg, days, h, mi, s, _us) => {
            let total_h = u64::from(days) * 24 + u64::from(h);
            let sign = if neg { "-" } else { "" };
            Some(format!("{sign}{total_h:02}:{mi:02}:{s:02}"))
        }
    }
}

fn rows_to_data(rows: Vec<mysql_async::Row>) -> Vec<RowData> {
    rows.into_iter().map(row_to_data).collect()
}

fn row_to_data(mut row: mysql_async::Row) -> RowData {
    (0..row.len())
        .map(|i| {
            let val: mysql_async::Value = row.take(i).unwrap_or(mysql_async::Value::NULL);
            mysql_value_to_string(val)
        })
        .collect()
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

impl MySqlRepo {
    async fn fetch_query_rows(
        &self,
        sql: &str,
        page: Option<SqlPage>,
    ) -> Result<FetchRowsResult, DbError> {
        let mut conn = self.conn.lock().await;
        let Some(page) = page else {
            let mut qr = conn.query_iter(sql).await.map_err(DbError::MySql)?;
            let columns: Vec<ColumnInfo> = qr
                .columns_ref()
                .iter()
                .map(|c| ColumnInfo { name: c.name_str().to_string() })
                .collect();
            let rows: Vec<mysql_async::Row> = qr.collect().await.map_err(DbError::MySql)?;
            let data = rows_to_data(rows);
            return Ok(FetchRowsResult {
                columns,
                total_count: data.len() as u64,
                rows: data,
            });
        };

        let (count_sql, data_sql) = build_pagination_sqls(sql, page.limit, page.offset);
        let counts: Vec<u64> = conn.query(&count_sql).await.map_err(DbError::MySql)?;
        let total_count = counts.into_iter().next().unwrap_or(0);

        let mut qr = conn.query_iter(&data_sql).await.map_err(DbError::MySql)?;
        let columns: Vec<ColumnInfo> = qr
            .columns_ref()
            .iter()
            .map(|c| ColumnInfo { name: c.name_str().to_string() })
            .collect();
        let rows: Vec<mysql_async::Row> = qr.collect().await.map_err(DbError::MySql)?;

        Ok(FetchRowsResult {
            columns,
            rows: rows_to_data(rows),
            total_count,
        })
    }
}

impl Database for MySqlRepo {
    async fn get_schemas(&self) -> Result<Vec<TableRef>, DbError> {
        let mut conn = self.conn.lock().await;
        let rows: Vec<mysql_async::Row> = conn
            .query(
                "SELECT table_schema, table_name
                 FROM information_schema.tables
                 WHERE table_type = 'BASE TABLE'
                   AND table_schema NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')
                 ORDER BY table_schema, table_name",
            )
            .await
            .map_err(DbError::MySql)?;
        Ok(rows
            .into_iter()
            .map(|mut r| TableRef {
                schema: r.take::<String, _>(0).unwrap_or_default(),
                name: r.take::<String, _>(1).unwrap_or_default(),
            })
            .collect())
    }

    async fn get_tables(&self, table_names: Vec<String>) -> Result<Vec<Table>, DbError> {
        if table_names.is_empty() {
            return Err(DbError::NotFound("Tables not found".to_string()));
        }
        let placeholders = table_names
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT DISTINCT table_name
             FROM information_schema.columns
             WHERE table_name IN ({placeholders})
             ORDER BY table_name"
        );
        let params = mysql_async::Params::Positional(
            table_names
                .iter()
                .map(|n| mysql_async::Value::from(n.as_str()))
                .collect(),
        );
        let mut conn = self.conn.lock().await;
        let rows: Vec<mysql_async::Row> = conn.exec(&sql, params).await.map_err(DbError::MySql)?;
        if rows.is_empty() {
            return Err(DbError::NotFound("Tables not found".to_string()));
        }
        Ok(rows
            .into_iter()
            .map(|mut r| Table {
                name: r.take::<String, _>(0).unwrap_or_default(),
            })
            .collect())
    }
    async fn execute_sql_with_options(
        &self,
        sql: &str,
        options: Option<SqlExecuteOptions>,
    ) -> Result<SqlExecuteResult, DbError> {
        if is_returning_query(sql) {
            let result = self
                .fetch_query_rows(sql, options.and_then(|o| o.page))
                .await?;
            return Ok(SqlExecuteResult::RowsReturned(result));
        }
        let mut conn = self.conn.lock().await;
        let qr = conn.query_iter(sql).await.map_err(DbError::MySql)?;
        let affected = qr.affected_rows();
        qr.drop_result().await.map_err(DbError::MySql)?;
        Ok(SqlExecuteResult::RowsAffected(affected))
    }
    async fn fetch_rows(
        &self,
        schema: &str,
        table: &str,
        limit: u16,
        offset: u64,
    ) -> Result<FetchRowsResult, DbError> {
        let esc_schema = schema.replace('`', "``");
        let esc_table = table.replace('`', "``");
        let mut conn = self.conn.lock().await;

        // Probe: get column metadata and detect id column
        let probe_sql = format!("SELECT * FROM `{esc_schema}`.`{esc_table}` LIMIT 0");
        let probe = conn.query_iter(&probe_sql).await.map_err(DbError::MySql)?;
        let has_id = probe.columns_ref().iter().any(|c| c.name_str() == "id");
        let columns: Vec<ColumnInfo> = probe
            .columns_ref()
            .iter()
            .map(|c| ColumnInfo { name: c.name_str().to_string() })
            .collect();
        probe.drop_result().await.map_err(DbError::MySql)?;

        // Count
        let count_sql = format!("SELECT COUNT(*) FROM `{esc_schema}`.`{esc_table}`");
        let counts: Vec<u64> = conn.query(count_sql).await.map_err(DbError::MySql)?;
        let total_count = counts.into_iter().next().unwrap_or(0);

        // Data
        let order = if has_id { " ORDER BY `id`" } else { "" };
        let data_sql = format!(
            "SELECT * FROM `{esc_schema}`.`{esc_table}`{order} LIMIT {limit} OFFSET {offset}"
        );
        let rows: Vec<mysql_async::Row> = conn.query(data_sql).await.map_err(DbError::MySql)?;

        Ok(FetchRowsResult {
            columns,
            rows: rows_to_data(rows),
            total_count,
        })
    }
    async fn get_table_details(
        &self,
        _schema: &str,
        _table: &str,
    ) -> Result<TableDetails, DbError> {
        todo!("MySQL get_table_details not yet implemented")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_returning_query_detects_select() {
        assert!(is_returning_query("SELECT 1"));
        assert!(is_returning_query("SELECT * FROM users"));
        assert!(is_returning_query("select id from posts where id = 1"));
    }

    #[test]
    fn is_returning_query_detects_explain() {
        assert!(is_returning_query("EXPLAIN SELECT 1"));
        assert!(is_returning_query("EXPLAIN SELECT * FROM users"));
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
    }

    #[test]
    fn is_returning_query_handles_invalid_sql() {
        assert!(!is_returning_query("NOT VALID SQL AT ALL"));
        assert!(!is_returning_query(""));
    }

    #[test]
    fn mysql_value_null_is_none() {
        assert_eq!(mysql_value_to_string(mysql_async::Value::NULL), None);
    }

    #[test]
    fn mysql_value_int_to_string() {
        assert_eq!(
            mysql_value_to_string(mysql_async::Value::Int(42)),
            Some("42".to_string())
        );
    }

    #[test]
    fn mysql_value_bytes_utf8_to_string() {
        assert_eq!(
            mysql_value_to_string(mysql_async::Value::Bytes(b"hello".to_vec())),
            Some("hello".to_string())
        );
    }

    #[test]
    fn mysql_value_bytes_invalid_utf8_returns_binary_placeholder() {
        let val = mysql_async::Value::Bytes(vec![0xFF, 0xFE]);
        assert_eq!(mysql_value_to_string(val), Some("<binary>".to_string()));
    }

    #[test]
    fn mysql_value_date_formats_as_datetime() {
        let val = mysql_async::Value::Date(2024, 3, 15, 10, 30, 45, 0);
        assert_eq!(
            mysql_value_to_string(val),
            Some("2024-03-15 10:30:45".to_string())
        );
    }

    fn test_config() -> crate::config::MySqlConfig {
        crate::config::MySqlConfig {
            name: None,
            host: std::env::var("TEST_MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string()),
            user: std::env::var("TEST_MYSQL_USER").unwrap_or_else(|_| "test_user".to_string()),
            db_name: std::env::var("TEST_MYSQL_DB").unwrap_or_else(|_| "db_test".to_string()),
            port: std::env::var("TEST_MYSQL_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3307),
            password: std::env::var("TEST_MYSQL_PASSWORD").ok(),
        }
    }

    #[tokio::test]
    async fn get_schemas() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let schemas = repo.get_schemas().await.unwrap();
        let users_entry = schemas
            .iter()
            .find(|s| s.schema == "db_test" && s.name == "users");
        assert!(users_entry.is_some(), "expected db_test.users in schemas");
    }

    #[tokio::test]
    async fn get_tables() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let tables = repo
            .get_tables(vec!["users".to_string(), "posts".to_string()])
            .await
            .unwrap();
        assert!(tables.iter().any(|t| t.name == "users"));
        assert!(tables.iter().any(|t| t.name == "posts"));
    }

    #[tokio::test]
    async fn full_db_get() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let schemas = repo.get_schemas().await.unwrap();
        let table_names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
        let tables = repo.get_tables(table_names).await.unwrap();
        assert!(tables.iter().any(|t| t.name == "users"));
        assert!(tables.iter().any(|t| t.name == "posts"));
    }

    #[tokio::test]
    async fn execute_sql_select_returns_rows() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo.execute_sql("SELECT 1 AS num").await.unwrap();
        match result {
            SqlExecuteResult::RowsReturned(r) => assert_eq!(r.total_count, 1),
            _ => panic!("Expected RowsReturned"),
        }
    }

    #[tokio::test]
    async fn execute_sql_select_multiple_rows() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo
            .execute_sql(
                "SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3 \
                 UNION ALL SELECT 4 UNION ALL SELECT 5",
            )
            .await
            .unwrap();
        match result {
            SqlExecuteResult::RowsReturned(r) => assert_eq!(r.total_count, 5),
            _ => panic!("Expected RowsReturned"),
        }
    }

    #[tokio::test]
    async fn execute_sql_update_returns_affected() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo
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
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo.execute_sql("SELECT * FROM nonexistent_table_xyz").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fetch_rows_returns_paginated_data() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo.fetch_rows("db_test", "users", 10, 0).await.unwrap();
        assert!(!result.columns.is_empty());
    }

    #[tokio::test]
    async fn fetch_rows_respects_limit_offset() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let page1 = repo.fetch_rows("db_test", "users", 1, 0).await.unwrap();
        let page2 = repo.fetch_rows("db_test", "users", 1, 1).await.unwrap();
        if page1.total_count > 1 {
            assert_ne!(page1.rows, page2.rows);
        }
    }

    #[tokio::test]
    async fn execute_sql_with_page_returns_select_results() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo
            .execute_sql_with_options(
                "SELECT 1 AS num, 'test' AS str",
                Some(SqlExecuteOptions {
                    page: Some(SqlPage { limit: 10, offset: 0 }),
                }),
            )
            .await
            .unwrap();
        let SqlExecuteResult::RowsReturned(r) = result else {
            panic!("Expected RowsReturned");
        };
        assert_eq!(r.columns.len(), 2);
        assert_eq!(r.rows.len(), 1);
    }
}
