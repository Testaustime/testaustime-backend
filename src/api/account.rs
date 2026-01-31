use axum::Json;
use http::StatusCode;
use serde_derive::Deserialize;
use utoipa::ToSchema;

use crate::{database::DatabaseWrapper, error::TimeError, models::UserIdentity};

/// Request body for updating account settings.
#[derive(Deserialize, ToSchema)]
pub struct Settings {
    /// Whether the user's profile should be publicly visible.
    public_profile: Option<bool>,
}

/// Update account settings.
///
/// Currently supports toggling public profile visibility.
#[utoipa::path(
    post,
    path = "/account/settings",
    request_body = Settings,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Settings updated"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn change_settings(
    user: UserIdentity,
    db: DatabaseWrapper,
    settings: Json<Settings>,
) -> Result<StatusCode, TimeError> {
    if let Some(public_profile) = settings.public_profile {
        db.change_visibility(user.id, public_profile).await?;
    };

    Ok(StatusCode::OK)
}
