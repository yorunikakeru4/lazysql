use crate::db::repo::db_repo::DbError;

/// Result of executing arbitrary SQL.
#[derive(Debug)]
pub enum SqlExecuteResult {
    RowsAffected(u64),
    RowsReturned(FetchRowsResult),
}

/// Options controlling arbitrary SQL execution.
#[derive(Debug, Default, Clone, Copy)]
pub struct SqlExecuteOptions {
    pub page: Option<SqlPage>,
}

/// Pagination options for SQL queries that return rows.
#[derive(Debug, Clone, Copy)]
pub struct SqlPage {
    pub limit: u16,
    pub offset: u64,
}

#[derive(Debug, PartialEq)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey(String), // referenced table name
    Unique,
}
pub fn parse_constraint(s: Option<&str>, referenced: Option<&str>) -> Option<ConstraintType> {
    match s {
        Some("PRIMARY KEY") => Some(ConstraintType::PrimaryKey),
        Some("UNIQUE") => Some(ConstraintType::Unique),
        Some("FOREIGN KEY") => referenced.map(|r| ConstraintType::ForeignKey(r.to_string())),
        _ => None,
    }
}
#[derive(Debug)]
pub struct TableRef {
    pub schema: String,
    pub name: String,
}

#[derive(Debug)]
pub struct Table {
    pub name: String,
}
#[derive(Debug)]
pub struct TableField {
    pub name: String,
    pub data_type: String,
    pub is_nullable: String,
    pub constraint: Option<ConstraintType>,
    pub default_value: Option<String>,
}

/// Metadata for a single index on a table.
#[derive(Debug)]
pub struct IndexInfo {
    pub name: String,
}

/// A table that holds a foreign key referencing this table.
#[derive(Debug)]
pub struct FkRef {
    pub from_table: String,
    pub column: String,
}

/// Rich schema metadata for a single table — used by the Inspect screen.
#[derive(Debug)]
pub struct TableDetails {
    pub name: String,
    pub schema: String,
    pub row_count: Option<i64>,
    pub size_pretty: Option<String>,
    pub fields: Vec<TableField>,
    pub indexes: Vec<IndexInfo>,
    pub fk_refs: Vec<FkRef>,
}

/// Column metadata for a result set.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
}

/// Row data as Vec<Option<String>> — None = NULL.
pub type RowData = Vec<Option<String>>;

/// Result of fetching rows with pagination.
#[derive(Debug)]
pub struct FetchRowsResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<RowData>,
    pub total_count: u64,
}

pub trait Database {
    async fn get_tables(&self, tables_names: Vec<String>) -> Result<Vec<Table>, DbError>;
    async fn get_schemas(&self) -> Result<Vec<TableRef>, DbError>;
    async fn execute_sql(&self, sql: &str) -> Result<SqlExecuteResult, DbError> {
        self.execute_sql_with_options(sql, None).await
    }
    async fn execute_sql_with_options(
        &self,
        sql: &str,
        options: Option<SqlExecuteOptions>,
    ) -> Result<SqlExecuteResult, DbError>;
    async fn fetch_rows(
        &self,
        schema: &str,
        table: &str,
        limit: u16,
        offset: u64,
    ) -> Result<FetchRowsResult, DbError>;
    async fn get_table_details(&self, schema: &str, table: &str) -> Result<TableDetails, DbError>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fetch_rows_result_stores_columns_and_rows() {
        let result = FetchRowsResult {
            columns: vec![ColumnInfo { name: "id".into() }],
            rows: vec![vec![Some("1".into())]],
            total_count: 1,
        };
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn column_info_holds_name() {
        let col = ColumnInfo {
            name: "email".into(),
        };
        assert_eq!(col.name, "email");
    }

    #[test]
    fn row_data_handles_nulls() {
        let row: RowData = vec![Some("value".into()), None, Some("other".into())];
        assert_eq!(row[0], Some("value".into()));
        assert_eq!(row[1], None);
    }
}
