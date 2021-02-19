pub struct ServerConfig {
    pub async_threads: usize,
    pub blocking_threads: usize,
    pub auth_threads: usize,
    pub port: u16,
    pub db_conn: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            async_threads: 2,
            blocking_threads: 128,
            auth_threads: 4,
            port: 5000,
            db_conn: "sqlite::memory:".to_string(),
        }
    }
}

pub enum AddConfig {
    User {
        db_url: String,
        username: String,
        password: String,
    },
}
