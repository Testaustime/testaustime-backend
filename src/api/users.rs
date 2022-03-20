use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    Responder,
};

use crate::{database::Database, models::RegisteredUser, requests::DataRequest, user::UserId};

#[get("/users/@me")]
pub async fn my_profile(user: RegisteredUser) -> Result<impl Responder> {
    Ok(web::Json(user))
}

#[get("/users/{username}/activity/data")]
pub async fn get_activities(
    data: Query<DataRequest>,
    path: Path<(String,)>,
    user: UserId,
    db: Data<Database>,
) -> Result<impl Responder> {
    if path.0 == "@me" {
        let data = block(move || db.get_activity(data.into_inner(), user.id).unwrap()).await?;
        Ok(web::Json(data))
    } else {
        let db_clone = db.clone();
        let friend_id = db_clone.get_user_by_name(&path.0)?.id;
        if friend_id == user.id {
            let data = db.get_activity(data.into_inner(), friend_id)?;
            Ok(web::Json(data))
        } else {
            match db.are_friends(user.id, friend_id) {
                Ok(b) => {
                    if b {
                        let data = db.get_activity(data.into_inner(), friend_id)?;
                        Ok(web::Json(data))
                    } else {
                        Err(ErrorUnauthorized("This user is not your friend"))
                    }
                }
                Err(e) => Err(ErrorInternalServerError(e)),
            }
        }
    }
}
