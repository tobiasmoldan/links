use warp::reject::Reject;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid command")]
    InvalidCommand,
    #[error("missing database connection string")]
    NoConnectionString,
    #[error("unauthorized")]
    Unauthorized,
    #[error("failed to decode auth header")]
    HeaderDecode,
    #[error("db error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("unknown error")]
    Unknown,
    #[error("{0}")]
    Custom(&'static str),
}

impl Reject for Error {}

impl Error {
    pub fn into_rejection(self) -> warp::Rejection {
        warp::reject::custom(self)
    }
}
