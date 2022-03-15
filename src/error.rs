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
}
