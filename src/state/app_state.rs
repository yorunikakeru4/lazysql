use crate::config::Connect;
use crate::config::Connect::Postgres;
use crate::db::repo::db_repo::{DbClient, DbError};
#[derive(Debug)]
struct AppState {
    current_db: Option<DbClient>, // Option потому что ну изначально-то нет никаого подключения
    connections: Vec<Connect>, // TODO: Наверное нужно будет подтягивать их из какого-нибудь config.toml
}

impl AppState {
    // изначально никакого подключения нет
    pub fn new(connections: Vec<Connect>) -> Self {
        AppState {
            current_db: None,
            connections,
        }
    }
    pub async fn connect(&mut self, idx: usize) -> Result<(), DbError> {
        let config = &self.connections[idx];
        match config {
            Postgres(_) => {
                let db_client = DbClient::new(config.clone()).await?;
                self.current_db = Some(db_client);
                Ok(())
            }
        }
    }
}
