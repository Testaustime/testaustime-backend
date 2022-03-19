use actix_web::{
    error::*,
    web::{self, Data},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;

use crate::{database::Database, error::TimeError, user::UserId};

#[post("/friends/add")]
pub async fn add_friend(user: UserId, body: String, db: Data<Database>) -> Result<impl Responder> {
    match db.add_friend(user.id, &body.trim().trim_start_matches("ttfc_")) {
        // This is not correct
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => ErrorConflict(e),
                _ => ErrorInternalServerError(e),
            })
        }
        Ok(name) => Ok(HttpResponse::Ok().body(name)),
    }
}

#[get("/friends/list")]
pub async fn get_friends(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.get_friends(user.id) {
        Ok(friends) => Ok(web::Json(friends)),
        Err(e) => {
            error!("{}", e);
            Err(ErrorInternalServerError(e))
        }
    }
}

#[post("/friends/regenerate")]
pub async fn regenerate_friend_code(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.regenerate_friend_code(user.id) {
        Ok(code) => Ok(HttpResponse::Ok().body(String::from("ttfc_") + &code)),
        Err(e) => {
            error!("{}", e);
            Err(ErrorInternalServerError(e))
        }
    }
}

#[delete("/friends/remove")]
pub async fn remove(user: UserId, db: Data<Database>, body: String) -> Result<impl Responder> {
    let friend = db.get_user_by_name(&body)?;
    let deleted = db.remove_friend(
        user.id,
        friend.id,
    )?;
    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ErrorBadRequest("Invalid id or Unauthorized"))
    }
}
