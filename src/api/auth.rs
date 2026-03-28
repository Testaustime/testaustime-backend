use std::sync::Arc;

use axum::{
    Json,
    extract::{FromRequestParts, State},
};
use chrono::{Duration, Local};
use http::{StatusCode, request::Parts};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    PasswordReset, PasswordResetState,
    api::users::UserAuthentication,
    auth::Authentication,
    database::DatabaseWrapper,
    error::TimeError,
    models::{NewUserIdentity, SelfUser, UserId, UserIdentity},
    utils::{generate_password_reset_token, validate_email},
};

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = TimeError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<Authentication>()
            .cloned()
            .expect("BUG: Every request should contain authentication, middleware issue");

        if let Authentication::AuthToken(user) = auth {
            Ok(UserId { id: user.id })
        } else {
            Err(TimeError::Unauthorized)
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for UserIdentity {
    type Rejection = TimeError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<Authentication>()
            .cloned()
            .expect("BUG: Every request should contain authentication, middleware issue");

        if let Authentication::AuthToken(user) = auth {
            Ok(user)
        } else {
            Err(TimeError::Unauthorized)
        }
    }
}

pub struct UserIdentityOptional {
    pub identity: Option<UserIdentity>,
}

impl<S: Send + Sync> FromRequestParts<S> for UserIdentityOptional
where
    S: Send + Sync,
{
    type Rejection = TimeError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<Authentication>()
            .cloned()
            .expect("BUG: Every request should contain authentication, middleware issue");

        if let Authentication::AuthToken(user) = auth {
            Ok(UserIdentityOptional {
                identity: Some(user),
            })
        } else {
            Ok(UserIdentityOptional { identity: None })
        }
    }
}

/// Request body for user login.
#[derive(Deserialize, Debug, ToSchema)]
pub struct LoginRequest {
    /// The user's username.
    pub username: String,
    /// The user's password.
    pub password: String,
}

/// Authenticate user with username and password.
///
/// Returns the user's profile including their authentication token on successful login.
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = OK, description = "Successfully authenticated", body = SelfUser),
        (status = 400, description = "Password too long"),
        (status = 401, description = "Invalid credentials"),
    )
)]
pub async fn login(
    db: DatabaseWrapper,
    data: Json<LoginRequest>,
) -> Result<Json<SelfUser>, TimeError> {
    if data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password cannot be longer than 128 characters".to_string(),
        ));
    }
    match db
        .verify_user_password(&data.username, &data.password)
        .await
    {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        _ => Err(TimeError::InvalidCredentials),
    }
}

/// Request body for user registration.
#[derive(Deserialize, Debug, ToSchema)]
pub struct RegisterRequest {
    /// Desired username (2-32 alphanumeric characters).
    pub username: String,
    /// Optional email address for account recovery.
    pub email: Option<String>,
    /// Password (8-128 characters).
    pub password: String,
}

/// Create a new Testaustime account.
///
/// Username must be 2-32 alphanumeric characters. Password must be 8-128 characters.
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = OK, description = "Account created successfully", body = NewUserIdentity),
        (status = 400, description = "Invalid username or password length"),
        (status = 409, description = "Username already taken"),
    )
)]
pub async fn register(
    db: DatabaseWrapper,
    Json(data): Json<RegisterRequest>,
) -> Result<Json<NewUserIdentity>, TimeError> {
    if data.password.len() < 8 || data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    if !super::VALID_NAME_REGEX.is_match(&data.username) {
        return Err(TimeError::BadUsername);
    }

    if data.email.as_ref().is_some_and(|e| validate_email(e)) {
        return Err(TimeError::InvalidEmail);
    }

    let username = data.username.clone();
    if db.get_user_by_name(&username).await.is_ok() {
        return Err(TimeError::UsernameTaken);
    }

    let res = db
        .new_testaustime_user(&data.username, &data.password, data.email.as_deref())
        .await?;

    Ok(Json(res))
}

/// Response containing the newly generated authentication token.
#[derive(Serialize, ToSchema)]
pub struct RegenerateResponse {
    /// The new authentication token.
    pub token: String,
}

/// Generate a new authentication token.
///
/// Requires username and password verification. Invalidates the previous token.
#[utoipa::path(
    post,
    path = "/auth/regenerate",
    request_body = UserAuthentication,
    responses(
        (status = OK, description = "New token generated", body = RegenerateResponse),
        (status = 401, description = "Invalid credentials"),
    )
)]
pub async fn regenerate_auth_token(
    db: DatabaseWrapper,
    Json(credentials): Json<UserAuthentication>,
) -> Result<Json<RegenerateResponse>, TimeError> {
    if let Some(user) = db
        .verify_user_password(&credentials.username, &credentials.password)
        .await?
    {
        db.regenerate_token(user.id)
            .await
            .inspect_err(|e| error!("{}", e))
            .map(|token| Json(RegenerateResponse { token }))
    } else {
        Err(TimeError::Unauthorized)
    }
}

/// Request body for changing username.
#[derive(Deserialize, ToSchema)]
pub struct UsernameChangeRequest {
    /// The new username (2-32 alphanumeric characters).
    pub new: String,
}

/// Update the authenticated user's username.
///
/// New username must be 2-32 alphanumeric characters and not already taken.
#[utoipa::path(
    post,
    path = "/auth/change-username",
    request_body = UsernameChangeRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Username changed successfully"),
        (status = 400, description = "Invalid username format or length"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Username already taken"),
    )
)]
pub async fn change_username(
    user: UserIdentity,
    db: DatabaseWrapper,
    Json(data): Json<UsernameChangeRequest>,
) -> Result<StatusCode, TimeError> {
    if data.new.len() < 2 || data.new.len() > 32 {
        return Err(TimeError::InvalidLength(
            "Username is not between 2 and 32 chars".to_string(),
        ));
    }

    if !super::VALID_NAME_REGEX.is_match(&data.new) {
        return Err(TimeError::BadUsername);
    }

    let result = db.change_username(user.id, &data.new).await;

    if result.as_ref().is_err_and(|e| e.is_unique_violation()) {
        return Err(TimeError::UsernameTaken);
    }

    result.map(|_| StatusCode::OK)
}

/// Request body for changing email address.
#[derive(Deserialize, ToSchema)]
pub struct EmailChangeRequest {
    /// The new email address.
    pub new: String,
}

/// Update the authenticated user's email address.
#[utoipa::path(
    post,
    path = "/auth/change-email",
    request_body = EmailChangeRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Email changed successfully"),
        (status = 400, description = "Invalid email format"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Email already taken"),
    )
)]
pub async fn change_email(
    user: UserIdentity,
    db: DatabaseWrapper,
    Json(data): Json<EmailChangeRequest>,
) -> Result<StatusCode, TimeError> {
    if !validate_email(&data.new) {
        return Err(TimeError::InvalidEmail);
    }

    let result = db.change_email(user.id, data.new).await;

    if result.as_ref().is_err_and(|e| e.is_unique_violation()) {
        return Err(TimeError::EmailTaken);
    }

    result.map(|_| StatusCode::OK)
}

/// Request body for changing password.
#[derive(Deserialize, ToSchema)]
pub struct PasswordChangeRequest {
    /// The current password for verification.
    pub old: String,
    /// The new password (8-128 characters).
    pub new: String,
}

/// Update the authenticated user's password.
///
/// Requires the current password for verification. New password must be 8-128 characters.
#[utoipa::path(
    post,
    path = "/auth/change-password",
    request_body = PasswordChangeRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Password changed successfully"),
        (status = 400, description = "Invalid password length"),
        (status = 401, description = "Unauthorized or incorrect current password"),
    )
)]
pub async fn change_password(
    user: UserIdentity,
    db: DatabaseWrapper,
    Json(body): Json<PasswordChangeRequest>,
) -> Result<StatusCode, TimeError> {
    if body.new.len() < 8 || body.new.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }

    let k = db.verify_user_password(&user.username, &body.old).await?;

    if k.is_some() {
        db.change_password(user.id, &body.new).await?;
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::Unauthorized)
    }
}

/// Request body for initiating password reset.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PasswordResetRequest {
    /// The email address associated with the account.
    pub email: String,
}

/// Request a password reset email.
///
/// Sends a password reset link to the user's email if the account exists.
/// Always returns OK to prevent email enumeration.
#[utoipa::path(
    post,
    path = "/auth/reset-password",
    request_body = PasswordResetRequest,
    responses(
        (status = OK, description = "Password reset email sent if account exists"),
    )
)]
pub async fn request_password_reset(
    db: DatabaseWrapper,
    State(password_resets): State<Arc<PasswordResetState>>,
    State(relay): State<AsyncSmtpTransport<Tokio1Executor>>,
    Json(body): Json<PasswordResetRequest>,
) -> Result<StatusCode, TimeError> {
    let Some(user) = db.get_user_by_email(&body.email).await? else {
        return Ok(StatusCode::OK);
    };

    let Some(ref email) = user.email else {
        return Ok(StatusCode::OK);
    };

    let token = generate_password_reset_token();

    let message = Message::builder()
        .from("Testaustime <noreply@testaustime.fi>".parse().expect("BUG: Infallible, email is hardcoded"))
        .to(format!("{} <{}>", user.username, email).parse().map_err(|_| TimeError::InvalidEmail)?)
        .subject("Testaustime password reset")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Here is your testaustime password reset link: https://testaustime.fi/reset_password?token={token}"))
        .expect("BUG: Infallible, everything is hardcoded and tested");

    tokio::spawn(async move {
        if let Err(err) = relay.send(message).await {
            error!("{}", err);
        };
    });

    debug!("Sent password reset of user {} to {}", user.username, email);

    password_resets.storage.insert(
        token,
        PasswordReset {
            expires: Local::now().naive_local() + Duration::minutes(30),
            user,
        },
    );

    Ok(StatusCode::OK)
}

/// Request body for completing password reset.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PasswordResetCompletionRequest {
    /// The new password to set.
    password: String,
    /// The reset token received via email.
    token: String,
}

/// Complete password reset with token.
///
/// Completes the password reset process using the token from the reset email.
#[utoipa::path(
    post,
    path = "/auth/complete-password-reset",
    request_body = PasswordResetCompletionRequest,
    responses(
        (status = OK, description = "Password reset successfully"),
        (status = 400, description = "Invalid or expired token"),
    )
)]
pub async fn reset_password(
    db: DatabaseWrapper,
    State(password_resets): State<Arc<PasswordResetState>>,
    Json(body): Json<PasswordResetCompletionRequest>,
) -> Result<StatusCode, TimeError> {
    let Some(reset) = password_resets.storage.get(&body.token) else {
        return Err(TimeError::InvalidPasswordResetToken);
    };

    if reset.expires > Local::now().naive_local() {
        db.change_password(reset.user.id, &body.password).await?;
        debug!("Changed password for user {}", reset.user.username);

        drop(reset);
        password_resets.storage.remove(&body.token);

        Ok(StatusCode::OK)
    } else {
        drop(reset);
        password_resets.storage.remove(&body.token);

        Err(TimeError::ExpiredPasswordResetToken)
    }
}
