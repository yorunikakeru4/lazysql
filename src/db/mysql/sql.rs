use mysql_async::prelude::*;
use sqlparser::ast::Statement;
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use crate::db::mysql::init::MySqlRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::sql_repo::{
    ColumnInfo, Database, FetchRowsResult, FkRef, IndexInfo, RowData, SqlExecuteOptions,
    SqlExecuteResult, SqlPage, Table, TableDetails, TableField, TableRef, parse_constraint,
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

/// Builds data_sql for pagination. Handles CTEs via MySQL dialect.
fn build_pagination_sqls(sql: &str, limit: u16, offset: u64) -> String {
    if let Some((with_str, body)) = try_extract_cte(sql) {
        format!(
            "{with_str}, _lazysql_data AS ({body}) \
             SELECT * FROM _lazysql_data LIMIT {limit} OFFSET {offset}"
        )
    } else {
        format!("SELECT * FROM ({sql}) AS _subq LIMIT {limit} OFFSET {offset}")
    }
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
        let page = page.unwrap_or(SqlPage {
            limit: 100,
            offset: 0,
        });

        let mut conn = self.conn.lock().await;

        let data_sql = build_pagination_sqls(sql, page.limit, page.offset);

        let mut qr = conn.query_iter(&data_sql).await.map_err(DbError::MySql)?;

        let columns: Vec<ColumnInfo> = qr
            .columns_ref()
            .iter()
            .map(|c| ColumnInfo {
                name: c.name_str().to_string(),
            })
            .collect();

        let rows: Vec<mysql_async::Row> = qr.collect().await.map_err(DbError::MySql)?;
        let data = rows_to_data(rows);

        Ok(FetchRowsResult {
            columns,
            total_count: page.offset + data.len() as u64,
            rows: data,
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

        // 1. Column metadata + detect id column
        let probe_sql = format!("SELECT * FROM `{esc_schema}`.`{esc_table}` LIMIT 0");
        let probe = conn.query_iter(&probe_sql).await.map_err(DbError::MySql)?;

        let has_id = probe.columns_ref().iter().any(|c| c.name_str() == "id");

        let columns: Vec<ColumnInfo> = probe
            .columns_ref()
            .iter()
            .map(|c| ColumnInfo {
                name: c.name_str().to_string(),
            })
            .collect();

        probe.drop_result().await.map_err(DbError::MySql)?;

        // 2. Approximate row count, fast for huge InnoDB tables
        let count_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT table_rows
             FROM information_schema.tables
             WHERE table_schema = ? AND table_name = ?",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;

        let total_count = count_rows
            .into_iter()
            .next()
            .and_then(|mut r| r.take::<Option<u64>, _>(0).flatten())
            .unwrap_or(0);

        // 3. Data
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

    /// Fetches detailed metadata for a table, including columns with constraints, row count, size, indexes, and inbound FK refs.
    async fn get_table_details(&self, schema: &str, table: &str) -> Result<TableDetails, DbError> {
        let mut conn = self.conn.lock().await;

        // 1. Columns + constraints (one row per column — correlated subqueries avoid
        //    the fan-out caused by MySQL's shared 'PRIMARY' constraint name across tables)
        let col_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT
                     c.column_name,
                     c.data_type,
                     c.is_nullable,
                     (SELECT tc2.constraint_type
                      FROM information_schema.key_column_usage kcu2
                      JOIN information_schema.table_constraints tc2
                          ON  tc2.constraint_name = kcu2.constraint_name
                          AND tc2.table_schema    = kcu2.table_schema
                          AND tc2.table_name      = kcu2.table_name
                      WHERE kcu2.table_schema = c.table_schema
                        AND kcu2.table_name   = c.table_name
                        AND kcu2.column_name  = c.column_name
                      ORDER BY CASE tc2.constraint_type
                          WHEN 'PRIMARY KEY' THEN 1
                          WHEN 'UNIQUE'      THEN 2
                          WHEN 'FOREIGN KEY' THEN 3
                          ELSE 4 END ASC
                      LIMIT 1) AS constraint_type,
                     (SELECT kcu2.referenced_table_name
                      FROM information_schema.key_column_usage kcu2
                      JOIN information_schema.table_constraints tc2
                          ON  tc2.constraint_name = kcu2.constraint_name
                          AND tc2.table_schema    = kcu2.table_schema
                          AND tc2.table_name      = kcu2.table_name
                      WHERE kcu2.table_schema   = c.table_schema
                        AND kcu2.table_name     = c.table_name
                        AND kcu2.column_name    = c.column_name
                        AND tc2.constraint_type = 'FOREIGN KEY'
                      LIMIT 1) AS referenced_table_name,
                     c.column_default
                 FROM information_schema.columns c
                 WHERE c.table_schema = ? AND c.table_name = ?
                 ORDER BY c.ordinal_position",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;

        let fields: Vec<TableField> = col_rows
            .into_iter()
            .map(|mut r| TableField {
                name: r.take::<String, _>(0).unwrap_or_default(),
                data_type: r.take::<String, _>(1).unwrap_or_default(),
                is_nullable: r.take::<String, _>(2).unwrap_or_default(),
                constraint: parse_constraint(
                    r.take::<Option<String>, _>(3).flatten().as_deref(),
                    r.take::<Option<String>, _>(4).flatten().as_deref(),
                ),
                default_value: r.take::<Option<String>, _>(5).flatten(),
            })
            .collect();

        // 2. Row count (exact)
        let count_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT table_rows
         FROM information_schema.tables
         WHERE table_schema = ? AND table_name = ?",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;

        let row_count: Option<i64> = count_rows
            .into_iter()
            .next()
            .and_then(|mut r| r.take::<Option<i64>, _>(0).flatten());

        // 3. Size (data + indexes in bytes → human-readable)
        let size_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT data_length + index_length
                 FROM information_schema.tables
                 WHERE table_schema = ? AND table_name = ?",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;
        let size_pretty: Option<String> = size_rows
            .into_iter()
            .next()
            .and_then(|mut r| r.take::<u64, _>(0))
            .map(format_bytes);

        // 4. Indexes
        let idx_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT DISTINCT index_name
                 FROM information_schema.statistics
                 WHERE table_schema = ? AND table_name = ?",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;
        let indexes: Vec<IndexInfo> = idx_rows
            .into_iter()
            .map(|mut r| IndexInfo {
                name: r.take::<String, _>(0).unwrap_or_default(),
            })
            .collect();

        // 5. Inbound FK refs (tables that reference this table)
        let fk_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT kcu.table_name, kcu.column_name
                 FROM information_schema.key_column_usage kcu
                 JOIN information_schema.table_constraints tc
                     ON tc.constraint_name = kcu.constraint_name
                     AND tc.table_schema   = kcu.table_schema
                 WHERE tc.constraint_type             = 'FOREIGN KEY'
                   AND kcu.referenced_table_schema    = ?
                   AND kcu.referenced_table_name      = ?",
                (schema.to_string(), table.to_string()),
            )
            .await
            .map_err(DbError::MySql)?;
        let fk_refs: Vec<FkRef> = fk_rows
            .into_iter()
            .map(|mut r| FkRef {
                from_table: r.take::<String, _>(0).unwrap_or_default(),
                column: r.take::<String, _>(1).unwrap_or_default(),
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
        let result = repo
            .execute_sql("SELECT * FROM nonexistent_table_xyz")
            .await;
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
    async fn get_table_details_returns_fields_and_indexes() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let details = repo.get_table_details("db_test", "users").await.unwrap();
        assert_eq!(details.name, "users");
        assert_eq!(details.schema, "db_test");
        assert!(!details.fields.is_empty(), "users table must have fields");
        assert!(
            !details.indexes.is_empty(),
            "users table must have at least PRIMARY KEY index"
        );
    }

    #[tokio::test]
    async fn get_table_details_includes_fk_refs_for_parent_table() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let details = repo.get_table_details("db_test", "users").await.unwrap();
        // posts.user_id references users.id → users should have an inbound FK ref
        assert!(
            details.fk_refs.iter().any(|fk| fk.from_table == "posts"),
            "users should have inbound FK ref from posts"
        );
    }

    #[tokio::test]
    async fn execute_sql_with_page_returns_select_results() {
        let repo = MySqlRepo::new(test_config()).await.unwrap();
        let result = repo
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
        let SqlExecuteResult::RowsReturned(r) = result else {
            panic!("Expected RowsReturned");
        };
        assert_eq!(r.columns.len(), 2);
        assert_eq!(r.rows.len(), 1);
    }
}
