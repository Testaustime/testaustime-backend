use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    HttpResponse, Responder,
};
use chrono::{Duration, Local};
use serde_derive::Deserialize;

use crate::{
    database::DatabaseConnection,
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
        block(move || db.get()?.get_user_leaderboards(user.id)).await??,
    ))
}

#[delete("/users/@me/delete")]
pub async fn delete_user(
    pool: Data<DbPool>,
    user: web::Json<UserAuthentication>,
) -> Result<impl Responder, TimeError> {
    let mut conn = pool.get()?;
    if let Some(user) = block(move || {
        pool.get()?
            .verify_user_password(&user.username, &user.password)
    })
    .await??
    {
        block(move || conn.delete_user(user.id)).await??;
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
        block(move || conn.get_activity(data.into_inner(), user.id)).await??
    } else {
        //FIXME: This is technically not required when the username equals the username of the
        //authenticated user
        let target_user = conn.get_user_by_name(&path.0)?;

        if target_user.id == user.id
            || target_user.is_public
            || block(move || conn.are_friends(user.id, target_user.id)).await??
        {
            block(move || db.get()?.get_activity(data.into_inner(), target_user.id)).await??
        } else {
            return Err(TimeError::Unauthorized);
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
        block(move || conn.get_all_activity(user.id)).await??
    } else {
        let target_user = conn.get_user_by_name(&path.0)?;

        if target_user.id == user.id
            || target_user.is_public
            || block(move || db.get()?.are_friends(user.id, target_user.id)).await??
        {
            block(move || conn.get_all_activity(target_user.id)).await??
        } else {
            return Err(TimeError::Unauthorized);
        }
    };

    //FIXME: This does a lot of unnecessary calculations
    let now = Local::now().naive_local();

    let all_time = group_by_language(data.clone().into_iter());
    let last_month = group_by_language(
        data.clone()
            .into_iter()
            .take_while(|d| now.signed_duration_since(d.start_time) < Duration::days(30)),
    );
    let last_week = group_by_language(
        data.into_iter()
            .take_while(|d| now.signed_duration_since(d.start_time) < Duration::days(7)),
    );

    let langs = serde_json::json!({
        "last_week": {
            "languages": last_week,
            "total": last_week.values().sum::<i32>(),
        },
        "last_month": {
            "languages": last_month,
            "total": last_month.values().sum::<i32>(),
        },
        "all_time": {
            "languages": all_time,
            "total": all_time.values().sum::<i32>(),
        },
    });

    Ok(web::Json(langs))
}
