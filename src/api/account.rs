use actix_web::{
    web::{self, block, Data},
    HttpResponse, Responder,
};
use serde_derive::Deserialize;

use crate::{database::Database, error::TimeError, models::UserId};

#[derive(Deserialize)]
pub struct Settings {
    public_profile: Option<bool>,
}

#[post("/account/settings")]
pub async fn change_settings(
    settings: web::Json<Settings>,
    userid: UserId,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    if let Some(public_profile) = settings.public_profile {
        block(move || db.get()?.change_visibility(userid.id, public_profile)).await??;
    };
    Ok(HttpResponse::Ok())
}
