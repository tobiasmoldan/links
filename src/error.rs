pub type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing database connection string")]
    NoConnectionString,
    #[error("db error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("unknown error")]
    Unknown,
}
