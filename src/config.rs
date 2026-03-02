#[derive(Debug)]
pub struct Connect {
    pub host: String,
    pub user: String,
    pub database: String,
    pub port: u32,
    pub password: Option<String>,
}
pub trait Parse {
    fn from(&self) -> String;
}

impl Parse for Connect {
    fn from(&self) -> String {
        let con = format!(
            "host={} user={} dbname={} port={}",
            self.host, self.user, self.database, self.port
        );
        if let Some(password) = &self.password {
            format!("{} password={}", con, password)
        } else {
            con
        }
    }
}
