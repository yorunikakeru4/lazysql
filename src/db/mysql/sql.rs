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

impl Database for MySqlRepo {
    async fn get_tables(&self, _table_names: Vec<String>) -> Result<Vec<Table>, DbError> {
        todo!("MySQL get_tables not yet implemented")
    }
    async fn get_schemas(&self) -> Result<Vec<TableRef>, DbError> {
        todo!("MySQL get_schemas not yet implemented")
    }
    async fn execute_sql_with_options(
        &self,
        _sql: &str,
        _options: Option<SqlExecuteOptions>,
    ) -> Result<SqlExecuteResult, DbError> {
        todo!("MySQL execute_sql not yet implemented")
    }
    async fn fetch_rows(
        &self,
        _schema: &str,
        _table: &str,
        _limit: u16,
        _offset: u64,
    ) -> Result<FetchRowsResult, DbError> {
        todo!("MySQL fetch_rows not yet implemented")
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
}
