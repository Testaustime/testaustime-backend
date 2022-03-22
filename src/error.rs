use actix_web::{
    error::{BlockingError, ResponseError},
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeError {
    #[error("Failed to connect to database connection pool")]
    R2d2Error(#[from] r2d2::Error),
    #[error("Diesel transaction failed `{0}`")]
    DieselError(#[from] diesel::result::Error),
    #[error(transparent)]
    DieselConnectionError(#[from] diesel::result::ConnectionError),
    #[error(transparent)]
    ActixError(#[from] actix_web::error::Error),
    #[error("User exists")]
    UserExistsError,
    #[error("User not found")]
    UserNotFound,
    #[error("You cannot add yourself")]
    CurrentUser,
    #[error("You are not authorized")]
    Unauthorized,
    #[error(transparent)]
    BlockingError(#[from] BlockingError),
    #[error("{0} is over {1} long")]
    TooLongError(String, i32),
    #[error("Bad id")]
    BadId,
}

impl ResponseError for TimeError {
    fn status_code(&self) -> StatusCode {
        error!("{}", self);
        match self {
            TimeError::BadId => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            _ => HttpResponse::build(self.status_code())
                .insert_header(ContentType::html())
                .body(json!({ "error": format!("{}", self) }).to_string()),
        }
    }
}
