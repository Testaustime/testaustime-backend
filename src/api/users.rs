use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    HttpResponse, Responder,
};
use chrono::{Duration, Local};
use serde_derive::Deserialize;

use crate::{
    database,
    error::TimeError,
    models::{UserId, UserIdentity},
    requests::DataRequest,
    utils::group_by_language,
    DbPool,
};

#[derive(Deserialize)]
pub struct UserAuthentication {
    pub username: String,
    pub password: String,
}

#[get("/users/@me")]
pub async fn my_profile(user: UserIdentity) -> Result<impl Responder, TimeError> {
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
        block(move || database::get_user_leaderboards(&mut db.get()?, user.id)).await??,
    ))
}

#[delete("/users/@me/delete")]
pub async fn delete_user(
    pool: Data<DbPool>,
    user: web::Json<UserAuthentication>,
) -> Result<impl Responder, TimeError> {
    let clone = pool.clone();
    if let Some(user) = block(move || {
        database::verify_user_password(&mut pool.get()?, &user.username, &user.password)
    })
    .await??
    {
        block(move || database::delete_user(&mut clone.get()?, user.id)).await??;
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
    let mut conn = db.get()?;

    let data = if path.0 == "@me" {
        block(move || database::get_activity(&mut conn, data.into_inner(), user.id)).await??
    } else {
        let friend_id = database::get_user_by_name(&mut conn, &path.0)?.id;

        if friend_id == user.id {
            let mut conn = db.get()?;
            block(move || database::get_activity(&mut conn, data.into_inner(), friend_id)).await??
        } else {
            if block(move || database::are_friends(&mut db.get()?, user.id, friend_id)).await?? {
                block(move || database::get_activity(&mut conn, data.into_inner(), friend_id))
                    .await??
            } else {
                return Err(TimeError::Unauthorized);
            }
        }
    };

    Ok(web::Json(data))
}

#[get("/users/{username}/activity/summary")]
pub async fn get_activity_summary(
    path: Path<(String,)>,
    user: UserId,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    let data = if path.0 == "@me" {
        block(move || database::get_all_activity(&mut conn, user.id)).await??
    } else {
        let friend_id = database::get_user_by_name(&mut conn, &path.0)?.id;

        if friend_id == user.id {
            let mut conn = db.get()?;
            block(move || database::get_all_activity(&mut conn, friend_id)).await??
        } else {
            if block(move || database::are_friends(&mut db.get()?, user.id, friend_id)).await?? {
                block(move || database::get_all_activity(&mut conn, friend_id)).await??
            } else {
                return Err(TimeError::Unauthorized);
            }
        }
    };

    let all_time = group_by_language(data.clone().into_iter());
    let last_month = group_by_language(data.clone().into_iter().take_while(|d| {
        Local::now()
            .naive_local()
            .signed_duration_since(d.start_time)
            < Duration::days(30)
    }));
    let last_week = group_by_language(data.clone().into_iter().take_while(|d| {
        Local::now()
            .naive_local()
            .signed_duration_since(d.start_time)
            < Duration::days(7)
    }));

    let langs = serde_json::json!({
        "last_week": last_week,
        "last_month": last_month,
        "all_time": all_time,
    });
    Ok(web::Json(langs))
}
