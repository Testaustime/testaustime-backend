use actix_web::{
    web::{Data, Json, Query},
    Responder,
};
use serde_derive::Deserialize;

use crate::{database::DatabaseConnection, error::TimeError, DbPool};

#[derive(Deserialize)]
pub struct UserSearch {
    pub keyword: String,
}

//TODO: Maybe return small coding summary?
#[get("/search/users")]
pub async fn search_public_users(
    db: Data<DbPool>,
    search: Query<UserSearch>,
) -> Result<impl Responder, TimeError> {
    Ok(Json(db.get()?.search_public_users(&search.keyword)?))
}
