use actix_web::{
    error::*,
    web::{self, Data},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;

use crate::{database::Database, error::TimeError, user::UserId};

#[post("/friends/add")]
pub async fn add_friend(
    user: UserId,
    body: String,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    match db.add_friend(user.id, &body.trim().trim_start_matches("ttfc_")) {
        // This is not correct
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => e,
                _ => e,
            })
        }
        Ok(name) => Ok(web::Json(json!({ "name": name }))),
    }
}

#[get("/friends/list")]
pub async fn get_friends(user: UserId, db: Data<Database>) -> Result<impl Responder, TimeError> {
    match db.get_friends(user.id) {
        Ok(friends) => Ok(web::Json(friends)),
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/friends/regenerate")]
pub async fn regenerate_friend_code(
    user: UserId,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    match db.regenerate_friend_code(user.id) {
        Ok(code) => Ok(web::Json(json!({
            "friend_code": format!("ttfc_{}", &code)
        }))),
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[delete("/friends/remove")]
pub async fn remove(
    user: UserId,
    db: Data<Database>,
    body: String,
) -> Result<impl Responder, TimeError> {
    let friend = db.get_user_by_name(&body)?;
    let deleted = db.remove_friend(user.id, friend.id)?;
    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::BadId)
    }
}
