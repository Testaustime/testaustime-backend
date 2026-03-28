use axum::{
    Json,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeError {
    #[error("Failed to connect to database connection pool")]
    DeadpoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("Diesel transaction failed `{0}`")]
    DieselError(#[from] diesel::result::Error),
    #[error("Internal server error")]
    DieselConnectionError(#[from] diesel::result::ConnectionError),
    #[error(transparent)]
    AxumError(#[from] axum::Error),
    #[error("User exists")]
    UsernameTaken,
    #[error("User not found")]
    UserNotFound,
    #[error("You cannot add yourself")]
    CurrentUser,
    #[error("Leaderboard exists")]
    LeaderboardExists,
    #[error("Leaderboard not found")]
    LeaderboardNotFound,
    #[error("You are not authorized")]
    Unauthorized,
    #[error("Invalid username or password")]
    InvalidCredentials,
    #[error("{0}")]
    InvalidLength(String),
    #[error(
        "Username has to contain characters from [a-zA-Z0-9_] and has to be between 2 and 32 characters"
    )]
    BadUsername,
    #[error(
        "Leaderboard name has to contain characters from [a-zA-Z0-9_] and has to be between 2 and 32 characters"
    )]
    BadLeaderboardName,
    #[error("Bad id")]
    BadId,
    #[error("Already friends")]
    AlreadyFriends,
    #[error("You're already a member")]
    AlreadyMember,
    #[error("You're not a member")]
    NotMember,
    #[error("There are no more admins left, you cannot leave")]
    LastAdmin,
    #[error("Bad code")]
    BadCode,
    #[error("Literally no idea how this happened")]
    UnknownError,
    #[error("You are trying to register again after a short time")]
    TooManyRegisters,
    #[error("The user has no active session")]
    NotActive,
    #[error("Cannot connect to mail server")]
    SmtpError(#[from] lettre::transport::smtp::Error),
    #[error("Invalid password reset token")]
    InvalidPasswordResetToken,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Expired password reset token")]
    ExpiredPasswordResetToken,
    #[error("Account with this email exists")]
    EmailTaken,
    #[error("Password hashing failed")]
    HashError(#[from] argon2::password_hash::Error),
    #[error(transparent)]
    HeaderError(#[from] http::header::ToStrError),
}

impl IntoResponse for TimeError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let status_code = match self {
            TimeError::UserNotFound | TimeError::LeaderboardNotFound | TimeError::NotActive => {
                StatusCode::NOT_FOUND
            }
            TimeError::BadUsername
            | TimeError::InvalidLength(_)
            | TimeError::BadId
            | TimeError::BadLeaderboardName
            | TimeError::InvalidEmail
            | TimeError::BadCode
            | TimeError::HeaderError(_) => StatusCode::BAD_REQUEST,
            TimeError::CurrentUser | TimeError::NotMember | TimeError::LastAdmin => {
                StatusCode::FORBIDDEN
            }
            TimeError::AlreadyFriends
            | TimeError::LeaderboardExists
            | TimeError::AlreadyMember
            | TimeError::UsernameTaken
            | TimeError::EmailTaken => StatusCode::CONFLICT,
            TimeError::Unauthorized
            | TimeError::InvalidCredentials
            | TimeError::InvalidPasswordResetToken
            | TimeError::ExpiredPasswordResetToken => StatusCode::UNAUTHORIZED,
            TimeError::TooManyRegisters => StatusCode::TOO_MANY_REQUESTS,
            TimeError::DieselError(_)
            | TimeError::DieselConnectionError(_)
            | TimeError::DeadpoolError(_)
            | Self::AxumError(_)
            | TimeError::UnknownError
            | TimeError::SmtpError(_)
            | TimeError::HashError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({"error": self.to_string()}));

        (status_code, body).into_response()
    }
}

impl TimeError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            TimeError::DieselError(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..
            ))
        )
    }
}
