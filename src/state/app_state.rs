use crate::config::Connect;
use crate::config::Connect::Postgres;
use crate::db::repo::db_repo::{DbClient, DbError};
#[derive(Debug)]
/// TODO: Наверное нужно будет подтягивать их из какого-нибудь **config.toml**, реализуем в отдельном extern crate storage, который будет читать и сохранять конфиги подключений, а также добавлять новые
pub struct AppState {
    // Option потому что ну изначально-то нет никаого подключения
    connections: Vec<Connect>,
    current_db: Option<DbClient>,
}

impl AppState {
    // изначально никакого подключения нет, но есть список доступных конфигураций для подключения
    pub fn new(connections: Vec<Connect>) -> Self {
        AppState {
            connections,
            current_db: None,
        }
    }
    /// Метод для подключения к базе данных по индексу из списка конфигураций, &mut self потому что мы будем менять состояние приложения, а не просто читать его, владение не передаем, так как мы не хотим, чтобы кто-то другой мог использовать это состояние после подключения, а также потому что мы не хотим, чтобы кто-то другой мог изменить это состояние после подключения
    pub async fn connect(&mut self, idx: usize) -> Result<(), DbError> {
        match &self.connections[idx] {
            Postgres(_) => {
                let db_client = DbClient::new(self.connections[idx].clone()).await?;
                self.current_db = Some(db_client);
            }
        }

        Ok(())
    }
    pub fn add(&mut self, connection: Connect) {
        self.connections.push(connection);
    }
}
