use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, State},
    Json,
};
use chrono::{Duration, Local};
use http::{request::Parts, StatusCode};
use lettre::{
    message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::Authentication,
    database::DatabaseWrapper,
    error::TimeError,
    models::{NewUserIdentity, SelfUser, UserId, UserIdentity},
    utils::{generate_password_reset_token, validate_email},
    PasswordReset, PasswordResetState,
};

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = TimeError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let auth = parts.extensions.get::<Authentication>().cloned().unwrap();

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
        let auth = parts.extensions.get::<Authentication>().cloned().unwrap();

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
        let auth = parts.extensions.get::<Authentication>().cloned().unwrap();

        if let Authentication::AuthToken(user) = auth {
            Ok(UserIdentityOptional {
                identity: Some(user),
            })
        } else {
            Ok(UserIdentityOptional { identity: None })
        }
    }
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    responses(
        (status = OK, body = SelfUser),
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

#[derive(Deserialize, Debug, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/auth/register",
    responses(
        (status = OK, body = NewUserIdentity)
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

#[derive(Deserialize, ToSchema)]
pub struct UsernameChangeRequest {
    pub new: String,
}

#[utoipa::path(
    post,
    path = "/auth/change-username",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
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

#[derive(Deserialize, ToSchema)]
pub struct EmailChangeRequest {
    pub new: String,
}

#[utoipa::path(
    post,
    path = "/auth/change-email",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
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

#[derive(Deserialize, ToSchema)]
pub struct PasswordChangeRequest {
    pub old: String,
    pub new: String,
}

#[utoipa::path(
    post,
    path = "/auth/change-password",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
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

    let testaustime_user = db.get_testaustime_user_by_id(user.id).await?;
    let k = db.verify_user_password(&user.username, &body.old).await?;

    if k.is_some() || testaustime_user.password.iter().all(|n| *n == 0) {
        db.change_password(user.id, &body.new).await?;
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[utoipa::path(
    post,
    path = "/auth/reset-password",
    responses(
        (status = OK)
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
        .from("Testaustime <noreply@testaustime.fi>".parse().unwrap())
        .to(format!("{} <{}>", user.username, email).parse().map_err(|_| TimeError::InvalidEmail)?)
        .subject("Testaustime password reset")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Here is your testaustime password reset link: https://testaustime.fi/reset_password?token={}", token)).unwrap();

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PasswordResetCompletionRequest {
    password: String,
    token: String,
}

#[utoipa::path(
    post,
    path = "/auth/complete-password-reset",
    responses(
        (status = OK)
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
