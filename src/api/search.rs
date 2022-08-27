use actix_web::{
    web::{Data, Json, Query},
    Responder,
};
use serde_derive::Deserialize;

use crate::{database, error::TimeError, DbPool};

#[derive(Deserialize)]
pub struct UserSearch {
    pub keyword: String,
}

//TODO: Maybe return small coding summary?
//FIXME: The error when missing `keyword` is ugly, go fix
#[get("/search/users")]
pub async fn search_public_users(
    db: Data<DbPool>,
    search: Query<UserSearch>,
) -> Result<impl Responder, TimeError> {
    Ok(Json(database::search_public_users(
        &mut db.get()?,
        &search.keyword,
    )?))
}
