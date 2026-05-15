use crate::db::mysql::init::MySqlRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::sql_repo::{
    Database, FetchRowsResult, SqlExecuteOptions, SqlExecuteResult, Table, TableDetails, TableRef,
};

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
