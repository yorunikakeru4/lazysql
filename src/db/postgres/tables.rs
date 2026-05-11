use crate::db::postgres::init::PostgresRepo;
use crate::db::repo::db_repo::DbError;
use crate::db::repo::tables_repo::{Database, Schema, Table, TableField, parse_constraint};
use std::collections::HashMap;

impl Database for PostgresRepo {
    // TODO: добавить помимо связуюшей таблицы, ещё и связующее поле (для определения связей между таблицами, например, для генерации ER диаграммы)
    async fn get_tables(&self, table_names: Vec<String>) -> Result<Vec<Table>, DbError> {
        let rows = self
            .client
            .query(
                "WITH tables_ AS (
                    SELECT unnest($1::text[]) AS table_name
                ),
                columns_ AS (
                    SELECT
                    isc.table_name,
                    isc.column_name,
                    isc.data_type,
                    isc.is_nullable,
                    isc.column_default AS default
                    FROM tables_ t
                    JOIN information_schema.columns isc ON isc.table_name = t.table_name
                ),
                constraints_ AS (
                    SELECT
                    kcu.table_name,
                    kcu.column_name,
                    tc.constraint_type,
                    c2.table_name AS referenced_table
                    FROM information_schema.key_column_usage kcu
                    JOIN information_schema.table_constraints tc
                    ON tc.constraint_name = kcu.constraint_name
                    LEFT JOIN information_schema.referential_constraints rc
                    ON rc.constraint_name = tc.constraint_name
                    LEFT JOIN information_schema.table_constraints c2
                    ON rc.unique_constraint_name = c2.constraint_name
                )
                    SELECT
                    c.table_name,
                    c.column_name,
                    c.data_type,
                    c.is_nullable,
                    cons.constraint_type AS constraint,
                    cons.referenced_table,
                    c.default
                        FROM columns_ c
                        LEFT JOIN constraints_ cons
                        ON cons.table_name = c.table_name AND cons.column_name = c.column_name
                        ORDER BY c.table_name, c.column_name;",
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

        let mut tables_info: HashMap<String, Vec<TableField>> = HashMap::new();
        for el in &rows {
            let table_name: String = el.get(0);
            let field = TableField {
                name: el.get(1),
                data_type: el.get(2),
                is_nullable: el.get(3),
                constraint_type: parse_constraint(el.get(4), el.get(5)),
                default: el.get(6),
            };
            tables_info.entry(table_name).or_default().push(field);
        }

        let tables = tables_info
            .into_iter()
            .map(|(name, fields)| Table { name, fields })
            .collect::<Vec<_>>();

        Ok(tables)
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
            .await
            .map_err(DbError::Postgres)?;
        let schemas = rows
            .into_iter()
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
    use super::*;

    fn test_config() -> crate::config::PostgresConfig {
        crate::config::PostgresConfig {
            host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            user: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "test_user".to_string()),
            db_name: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "db_test".to_string()),
            port: std::env::var("TEST_DB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            password: std::env::var("TEST_DB_PASSWORD").ok(),
        }
    }

    #[tokio::test]
    async fn get_schemas() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let users: Schema = client
            .get_schemas()
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.name == "users")
            .unwrap();
        assert_eq!(users.name, "users");
        assert_eq!(users.schema, "public");
        assert_eq!(users.catalog, "db_test");
    }

    #[tokio::test]
    async fn get_tables() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let table_names = vec!["users".to_string(), "posts".to_string()];
        verify_tables(client, table_names).await;
    }

    async fn verify_tables(client: PostgresRepo, table_names: Vec<String>) {
        let tables = client.get_tables(table_names).await.unwrap();
        let posts = tables.iter().find(|f| f.name == "posts").unwrap();
        let user_id = posts.fields.iter().find(|f| f.name == "user_id").unwrap();
        assert_eq!(user_id.data_type, "integer");
        assert_eq!(user_id.is_nullable, "NO");
        assert_eq!(
            user_id.constraint_type,
            Some(crate::db::repo::tables_repo::ConstraintType::ForeignKey(
                "users".to_string()
            ))
        );
    }

    #[tokio::test]
    async fn full_db_get() {
        let client = PostgresRepo::new(test_config()).await.unwrap();
        let schemas = client.get_schemas().await.unwrap();
        let table_names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
        verify_tables(client, table_names).await;
    }
}
