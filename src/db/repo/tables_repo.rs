use crate::db::repo::db_repo::DbError;

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

pub trait Database {
    async fn get_tables(&self, tables_names: Vec<String>) -> Result<Vec<Table>, DbError>;
    async fn get_schemas(&self) -> Result<Vec<Schema>, DbError>;
}
