use actix_web::{
    error::*,
    web::{self, Data},
    Responder,
};

use crate::{database::Database, user::UserId};

#[get("/users/@me")]
pub async fn my_profile(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.get_user_by_id(user) {
        Ok(user) => Ok(web::Json(user)),
        Err(e) => {
            error!("{}", e);
            Err(ErrorInternalServerError(e))
        }
    }
}
