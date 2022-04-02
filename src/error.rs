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
    #[error("Internal server error")]
    DieselConnectionError(#[from] diesel::result::ConnectionError),
    #[error(transparent)]
    ActixError(#[from] actix_web::error::Error),
    #[error("User exists")]
    UserExists,
    #[error("User not found")]
    UserNotFound,
    #[error("You cannot add yourself")]
    CurrentUser,
    #[error("You are not authorized")]
    Unauthorized,
    #[error(transparent)]
    BlockingError(#[from] BlockingError),
    #[error("{0}")]
    InvalidLength(String),
    #[error("Username has to contain characters from [a-zA-Z0-9_] and has to be between 2 and 32 characters")]
    BadUsername,
    #[error("Bad id")]
    BadId,
    #[error("Already friends")]
    AlreadyFriends,
}

unsafe impl Send for TimeError {}

impl ResponseError for TimeError {
    fn status_code(&self) -> StatusCode {
        error!("{}", self);
        match self {
            TimeError::BadUsername | TimeError::InvalidLength(_) | TimeError::BadId => {
                StatusCode::BAD_REQUEST
            }
            TimeError::CurrentUser | TimeError::UserExists | TimeError::AlreadyFriends => {
                StatusCode::CONFLICT
            }
            TimeError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(json!({ "error": format!("{}", self) }).to_string())
    }
}
