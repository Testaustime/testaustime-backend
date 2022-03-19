use actix_web::{error::ResponseError, http::StatusCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeError {
    #[error("Failed to connect to database connection pool")]
    R2d2Error(#[from] r2d2::Error),
    #[error("Diesel transaction failed `{0}`")]
    DieselError(#[from] diesel::result::Error),
    #[error("Diesel transaction failed `{0}`")]
    DieselConnectionError(#[from] diesel::result::ConnectionError),
    #[error("Failed to connect to database connection pool")]
    ActixError(#[from] actix_web::Error),
    #[error("User exists")]
    UserExistsError,
    #[error("User not found")]
    UserNotFound,
}

impl ResponseError for TimeError {
    fn status_code(&self) -> StatusCode {
        error!("{}", self);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
