use crate::db::repo::db_repo::DbError;

/// Result of executing arbitrary SQL.
#[derive(Debug)]
pub enum SqlExecuteResult {
    RowsAffected(u64),
    RowsReturned(usize),
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
#[allow(dead_code)]
pub struct Schema {
    pub catalog: String,
    pub schema: String,
    pub name: String,
}

#[derive(Debug)]
pub struct Table {
    pub name: String,
    pub fields: Vec<TableField>,
}
#[derive(Debug)]
pub struct TableField {
    pub name: String,
    pub data_type: String,
    pub is_nullable: String,
    pub constraint_type: Option<ConstraintType>,
    pub default: Option<String>,
}

/// Column metadata for a result set.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
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
    async fn get_schemas(&self) -> Result<Vec<Schema>, DbError>;
    async fn execute_sql(&self, sql: &str) -> Result<SqlExecuteResult, DbError>;
    async fn fetch_rows(
        &self,
        schema: &str,
        table: &str,
        limit: u16,
        offset: u64,
    ) -> Result<FetchRowsResult, DbError>;
    async fn execute_sql_with_rows(
        &self,
        sql: &str,
        limit: u16,
        offset: u64,
    ) -> Result<FetchRowsResult, DbError>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fetch_rows_result_stores_columns_and_rows() {
        let result = FetchRowsResult {
            columns: vec![ColumnInfo {
                name: "id".into(),
                data_type: "int4".into(),
            }],
            rows: vec![vec![Some("1".into())]],
            total_count: 1,
        };
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn column_info_holds_name_and_type() {
        let col = ColumnInfo {
            name: "email".into(),
            data_type: "varchar".into(),
        };
        assert_eq!(col.name, "email");
        assert_eq!(col.data_type, "varchar");
    }

    #[test]
    fn row_data_handles_nulls() {
        let row: RowData = vec![Some("value".into()), None, Some("other".into())];
        assert_eq!(row[0], Some("value".into()));
        assert_eq!(row[1], None);
    }
}
