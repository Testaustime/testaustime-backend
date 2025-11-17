use axum::Json;
use http::StatusCode;
use serde_derive::Deserialize;
use utoipa::ToSchema;

use crate::{database::DatabaseWrapper, error::TimeError, models::UserIdentity};

#[derive(Deserialize, ToSchema)]
pub struct Settings {
    public_profile: Option<bool>,
}

#[utoipa::path(
    post,
    path = "/account/settings",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
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
