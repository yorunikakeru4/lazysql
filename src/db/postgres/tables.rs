use crate::db::postgres::connection::PostgresRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::tables_repo::{Schema, Table, TableRepo};
impl TableRepo for PostgresRepo {
    async fn get_tables(&self) -> Result<Vec<Table>, DbError> {
        todo!()
    }

    async fn get_schemas(&self) -> Result<Vec<Schema>, DbError> {
        let rows = self
            .client
            .query(
                "
            SELECT table_catalog, table_schema, table_name
            FROM information_schema.tables
            WHERE table_type='BASE TABLE'
              AND table_schema NOT IN ('pg_catalog', 'information_schema');
            ",
                &[],
            )
            .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => return Err(DbError::Postgres(e)),
        };
        let schemas = rows
            .iter()
            .map(|x| Schema {
                catalog: x.get(0),
                schema: x.get(1),
                name: x.get(2),
            })
            .collect();

        Ok(schemas)
    }
}

#[cfg(test)]
mod test {
    use crate::db::repo::db_repo::Repo;

    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_get_schemas() {
        let config = crate::config::Connect {
            host: "localhost".to_string(),
            user: "test_user".to_string(),
            database: "db_test".to_string(),
            port: 5432,
            password: Some("vBnA46MVSs".to_string()),
        };

        let client = PostgresRepo::new(config).await;
        let rows = client.unwrap().get_schemas().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "users");
        assert_eq!(rows[0].schema, "public");
        assert_eq!(rows[0].catalog, "db_test");
    }
}
