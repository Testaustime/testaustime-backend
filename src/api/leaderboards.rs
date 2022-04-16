use actix_web::{
    error::*,
    web::{self, block, Data, Path, Query},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;
use serde_derive::Deserialize;

use crate::{database, error::TimeError, models::*, requests::DataRequest, user::UserId, DbPool};

#[post("/leaderboards")]
pub async fn create_leaderboard(
    creator: UserId,
    body: String,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    match block(move || database::new_leaderboard(&db.get()?, creator.id, &body)).await? {
        Ok(code) => Ok(web::Json(json!({
            "invite_code": format!("ttlic_{}", code)
        }))),
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::LeaderboardExists,
                _ => e,
            })
        }
    }
}

#[get("/leaderboards/{name}")]
pub async fn get_leaderboard(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    let name = path.0.clone();
    match block(move || database::is_leaderboard_member_by_lname(&conn, user.id, &name)).await? {
        Err(e) => {
            // this is not correct
            error!("{}", e);
            Err(TimeError::LeaderboardNotFound)
        }
        Ok(true) => Ok(web::Json({
            let conn = db.get()?;
            block(move || database::get_leaderboard(&conn, &path.0)).await??
        })),
        Ok(false) => Err(TimeError::Unauthorized),
    }
}

#[delete("/leaderboards/{name}")]
pub async fn delete_leaderboard(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    let name = path.0.clone();
    match block(move || database::is_leaderboard_admin_by_lname(&conn, user.id, &name)).await? {
        Err(e) => {
            // this is not correct
            error!("{}", e);
            Err(TimeError::LeaderboardNotFound)
        }
        Ok(true) => Ok(web::Json({
            let conn = db.get()?;
            block(move || database::delete_leaderboard(&conn, path.0.clone())).await??
        })),
        Ok(false) => Err(TimeError::Unauthorized),
    }
}

#[post("/leaderboards/join")]
pub async fn join_leaderboard(
    user: UserId,
    body: String,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    match block(move || {
        database::add_user_to_leaderboard(
            &db.get()?,
            user.id,
            body.trim().trim_start_matches("ttlic_"),
        )
    })
    .await?
    {
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::AlreadyMember,
                _ => e,
            })
        }
        Ok(name) => Ok(web::Json(json!({ "name": name }))),
    }
}

#[post("/leaderboards/{name}/leave")]
pub async fn leave_leaderboard(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &path.0)).await? {
        let left = block(move || database::remove_user_from_leaderboard(&db.get()?, user.id, lid))
            .await??;
        if left {
            Ok(HttpResponse::Ok().finish())
        } else {
            Err(TimeError::NotMember)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}

#[post("/leaderboards/{name}/promote")]
pub async fn promote_member(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    todo!();
}
