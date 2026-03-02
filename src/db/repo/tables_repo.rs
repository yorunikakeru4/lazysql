use crate::db::repo::db_repo::DbError;
#[derive(Debug)]
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
    pub table_type: String,
    pub constraint: Option<String>,
    pub referenced_table: Option<String>,
    pub default: Option<String>,
}

pub trait TableRepo {
    async fn get_tables(&self) -> Result<Vec<Table>, DbError>;
    async fn get_schemas(&self) -> Result<Vec<Schema>, DbError>;
}
