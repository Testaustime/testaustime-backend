use actix_web::{
    error::*,
    web::{self, block, Data, Json, Path},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;
use serde::Deserialize;

use crate::{database, error::TimeError, models::UserId, DbPool};

#[derive(Deserialize)]
pub struct LeaderboardName {
    pub name: String,
}

#[derive(Deserialize)]
pub struct LeaderboardInvite {
    pub invite: String,
}

#[derive(Deserialize)]
pub struct LeaderboardUser {
    pub user: String,
}

#[post("/leaderboards/create")]
pub async fn create_leaderboard(
    creator: UserId,
    body: Json<LeaderboardName>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    match block(move || database::new_leaderboard(&db.get()?, creator.id, &body.name)).await? {
        Ok(code) => Ok(web::Json(json!({ "invite_code": code }))),
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
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &name)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_member(&conn, user.id, lid)).await?? {
            Ok(web::Json({
                let conn = db.get()?;
                block(move || database::get_leaderboard(&conn, &path.0)).await??
            }))
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
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
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &name)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&conn, user.id, lid)).await?? {
            let conn = db.get()?;
            block(move || database::delete_leaderboard(&conn, &path.0)).await??;
            Ok(HttpResponse::Ok().finish())
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}

#[post("/leaderboards/join")]
pub async fn join_leaderboard(
    user: UserId,
    body: Json<LeaderboardInvite>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    match block(move || {
        database::add_user_to_leaderboard(
            &db.get()?,
            user.id,
            body.invite.trim().trim_start_matches("ttlic_"),
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
    promotion: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &path.0)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&conn, user.id, lid)).await?? {
            let conn = db.get()?;
            if let Ok(newadmin) =
                block(move || database::get_user_by_name(&conn, &promotion.user)).await?
            {
                let conn = db.get()?;
                if block(move || {
                    database::promote_user_to_leaderboard_admin(&conn, lid, newadmin.id)
                })
                .await??
                {
                    Ok(HttpResponse::Ok().finish())
                } else {
                    // FIXME: This is not correct
                    Err(TimeError::NotMember)
                }
            } else {
                error!("{}", TimeError::UserNotFound);
                Err(TimeError::UserNotFound)
            }
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}

#[post("/leaderboards/{name}/demote")]
pub async fn demote_member(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
    demotion: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &path.0)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&conn, user.id, lid)).await?? {
            let conn = db.get()?;
            if let Ok(oldadmin) =
                block(move || database::get_user_by_name(&conn, &demotion.user)).await?
            {
                let conn = db.get()?;
                if block(move || {
                    database::demote_user_to_leaderboard_member(&conn, lid, oldadmin.id)
                })
                .await??
                {
                    Ok(HttpResponse::Ok().finish())
                } else {
                    // FIXME: This is not correct
                    Err(TimeError::NotMember)
                }
            } else {
                error!("{}", TimeError::UserNotFound);
                Err(TimeError::UserNotFound)
            }
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}

#[post("/leaderboards/{name}/kick")]
pub async fn kick_member(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
    kick: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &path.0)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&conn, user.id, lid)).await?? {
            let conn = db.get()?;
            if let Ok(kmember) =
                block(move || database::get_user_by_name(&conn, &kick.user)).await?
            {
                let conn = db.get()?;
                if block(move || database::remove_user_from_leaderboard(&conn, lid, kmember.id))
                    .await??
                {
                    Ok(HttpResponse::Ok().finish())
                } else {
                    Err(TimeError::NotMember)
                }
            } else {
                error!("{}", TimeError::UserNotFound);
                Err(TimeError::UserNotFound)
            }
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}

#[post("/leaderboards/{name}/regenerate")]
pub async fn regenerate_invite(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&conn, &path.0)).await? {
        let conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&conn, user.id, lid)).await?? {
            let conn = db.get()?;
            let code = block(move || database::regenerate_leaderboard_invite(&conn, lid)).await??;
            Ok(web::Json(json!({ "invite_code": code })))
        } else {
            error!("{}", TimeError::Unauthorized);
            Err(TimeError::Unauthorized)
        }
    } else {
        error!("{}", TimeError::LeaderboardNotFound);
        Err(TimeError::LeaderboardNotFound)
    }
}
