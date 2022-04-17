use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    HttpResponse, Responder,
};
use serde_derive::Deserialize;

use crate::{
    database::{self, are_friends, get_activity, get_user_by_name},
    error::TimeError,
    models::RegisteredUser,
    requests::DataRequest,
    user::UserId,
    DbPool,
};

#[derive(Deserialize)]
pub struct UserAuthentication {
    pub username: String,
    pub password: String,
}

#[get("/users/@me")]
pub async fn my_profile(user: RegisteredUser) -> Result<impl Responder, TimeError> {
    Ok(web::Json(user))
}

#[derive(serde::Serialize)]
pub struct ListLeaderboard {
    pub name: String,
    pub member_count: i32,
}

#[get("/users/@me/leaderboards")]
pub async fn my_leaderboards(user: UserId, db: Data<DbPool>) -> Result<impl Responder, TimeError> {
    Ok(web::Json(
        block(move || database::get_user_leaderboards(&db.get()?, user.id)).await??,
    ))
}

#[delete("/users/@me/delete")]
pub async fn delete_user(
    pool: Data<DbPool>,
    user: web::Json<UserAuthentication>,
) -> Result<impl Responder, TimeError> {
    let clone = pool.clone();
    if let Some(user) =
        block(move || database::verify_user_password(&pool.get()?, &user.username, &user.password))
            .await??
    {
        block(move || database::delete_user(&clone.get()?, user.id)).await??;
    }
    Ok(HttpResponse::Ok().finish())
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
            })
            .await?
            {
                Ok(b) => {
                    if b {
                        let data = block(move || get_activity(&conn, data.into_inner(), friend_id))
                            .await??;
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
