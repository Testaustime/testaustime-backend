use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    Responder,
};
use crate::{
    database::{get_activity, get_user_by_name, are_friends}, error::TimeError, models::RegisteredUser, requests::DataRequest,
    user::UserId, DbPool,
};

#[get("/users/@me")]
pub async fn my_profile(user: RegisteredUser) -> Result<impl Responder, TimeError> {
    return Ok(web::Json(user));
}

#[get("/users/{username}/activity/data")]
pub async fn get_activities(
    data: Query<DataRequest>,
    path: Path<(String,)>,
    user: UserId,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if path.0 == "@me" {
        let data = block(move || get_activity(&conn, data.into_inner(), user.id)).await??;
        Ok(web::Json(data))
    } else {
        let friend_id = get_user_by_name(&conn, &path.0)?.id;
        if friend_id == user.id {
            let conn = db.get()?;
            let data = block(move || get_activity(&conn, data.into_inner(), friend_id)).await??;
            Ok(web::Json(data))
        } else {
            match block(move || {
                let conn = db.get()?;
                are_friends(&conn, user.id, friend_id)
            }).await? {
                Ok(b) => {
                    if b {
                        let data = block(move || get_activity(&conn, data.into_inner(), friend_id)).await??;
                        Ok(web::Json(data))
                    } else {
                        Err(TimeError::Unauthorized)
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}
