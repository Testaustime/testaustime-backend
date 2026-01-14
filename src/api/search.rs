use axum::{extract::Query, Json};
use serde_derive::Deserialize;
use utoipa::IntoParams;

use crate::{database::DatabaseWrapper, error::TimeError, models::PublicUser};

#[derive(Deserialize, IntoParams)]
pub struct UserSearch {
    pub keyword: String,
}

//TODO: Maybe return small coding summary?
#[utoipa::path(
    get,
    path = "/search/users",
    params(
        UserSearch
    ),
    responses(
        (status = OK, body = Vec<PublicUser>)
    )
)]
pub async fn search_public_users(
    db: DatabaseWrapper,
    search: Query<UserSearch>,
) -> Result<Json<Vec<PublicUser>>, TimeError> {
    Ok(Json(db.search_public_users(search.keyword.clone()).await?))
}
