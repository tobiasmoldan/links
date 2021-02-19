pub type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing database connection string")]
    NoConnectionString,
    #[error("db error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("unknown error")]
    Unknown,
}
