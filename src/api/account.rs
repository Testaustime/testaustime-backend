use actix_web::{Responder, web::{block, self, Data}, HttpResponse};

use crate::{DbPool, error::TimeError, database, models::UserId};

use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    public_profile: Option<bool>,
}

#[post("/account/settings")]
pub async fn change_settings(
    settings: web::Json<Settings>,
    userid: UserId,
    db: Data<DbPool>
) -> Result<impl Responder, TimeError> {
    if let Some(public_profile) = settings.public_profile {
        block(move || database::change_visibility(&mut db.get()?, userid.id, public_profile)).await??;
    };
    Ok(HttpResponse::Ok())
}
