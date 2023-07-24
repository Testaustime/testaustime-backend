use actix_web::{
    web::{Json, Query},
    Responder,
};
use serde_derive::Deserialize;

use crate::{database::DatabaseWrapper, error::TimeError};

#[derive(Deserialize)]
pub struct UserSearch {
    pub keyword: String,
}

//TODO: Maybe return small coding summary?
#[get("/search/users")]
pub async fn search_public_users(
    db: DatabaseWrapper,
    search: Query<UserSearch>,
) -> Result<impl Responder, TimeError> {
    Ok(Json(db.search_public_users(search.keyword.clone()).await?))
}
