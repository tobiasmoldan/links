use serde::{ser::SerializeMap, Serialize};
use warp::{hyper::StatusCode, reject::Reject};

pub type Result<T> = std::result::Result<T, ApplicationError>;
pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("invalid command")]
    InvalidCommand,
    #[error("missing database connection string")]
    NoConnectionString,
    #[error("db error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("{0}")]
    Custom(&'static str),
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("failed to decode auth header")]
    AuthHeaderDecode,
    #[error("{0}")]
    Custom(&'static str),
    #[error("path {0} already exists")]
    PathAlreadyExists(String),
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid uri {0}")]
    InvalidUri(String),
}

impl Reject for ApiError {}

#[derive(Debug)]
struct ApiErrorMessage {
    status_code: StatusCode,
    message: String,
}

impl Serialize for ApiErrorMessage {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serializer_map = serializer.serialize_map(Some(2))?;
        serializer_map.serialize_entry("code", &self.status_code.as_u16())?;
        serializer_map.serialize_entry("message", &self.message)?;
        serializer_map.end()
    }
}

impl From<&ApiError> for ApiErrorMessage {
    fn from(api_error: &ApiError) -> Self {
        let code = match api_error {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::AuthHeaderDecode => StatusCode::BAD_REQUEST,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::PathAlreadyExists(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = if shadow_rs::is_debug() {
            format!("{}", api_error)
        } else {
            code.to_string()
        };

        ApiErrorMessage {
            status_code: code,
            message,
        }
    }
}

impl warp::Reply for &ApiError {
    fn into_response(self) -> warp::reply::Response {
        let msg = ApiErrorMessage::from(self);
        warp::reply::with_status(warp::reply::json(&msg), msg.status_code).into_response()
    }
}

trait ErrorCode {
    fn error_code(&self) -> u16;
}
