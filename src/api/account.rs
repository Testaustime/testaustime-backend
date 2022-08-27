use actix_web::{
    web::{self, block, Data},
    HttpResponse, Responder,
};
use serde_derive::Deserialize;

use crate::{database, error::TimeError, models::UserId, DbPool};

#[derive(Deserialize)]
pub struct Settings {
    public_profile: Option<bool>,
}

#[post("/account/settings")]
pub async fn change_settings(
    settings: web::Json<Settings>,
    userid: UserId,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if let Some(public_profile) = settings.public_profile {
        block(move || database::change_visibility(&mut db.get()?, userid.id, public_profile))
            .await??;
    };
    Ok(HttpResponse::Ok())
}
