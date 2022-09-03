use actix_web::{
    error::*,
    web::{self, block, Data, Json, Path},
    HttpResponse, Responder,
};
use dashmap::DashMap;
use diesel::result::DatabaseErrorKind;
use serde::Deserialize;

use crate::{
    database,
    error::TimeError,
    models::{PrivateLeaderboard, UserId},
    DbPool,
};

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

pub struct CachedLeaderboard {
    pub board: PrivateLeaderboard,
    pub valid_until: chrono::DateTime<chrono::Utc>,
}

pub type LeaderboardCache = DashMap<i32, CachedLeaderboard>;

#[post("/leaderboards/create")]
pub async fn create_leaderboard(
    creator: UserId,
    body: Json<LeaderboardName>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if !super::VALID_NAME_REGEX.is_match(&body.name) {
        return Err(TimeError::BadLeaderboardName);
    }
    let mut conn = db.get()?;
    let lname = body.name.clone();
    if block(move || database::get_leaderboard_id_by_name(&mut conn, &lname))
        .await?
        .is_ok()
    {
        return Err(TimeError::LeaderboardExists);
    }

    match block(move || database::new_leaderboard(&mut db.get()?, creator.id, &body.name)).await? {
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
    cache: Data<LeaderboardCache>,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    let name = path.0.clone();
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &name)).await? {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_member(&mut conn, user.id, lid)).await?? {
            if let Some(cached_leaderboard) = cache.get(&lid) {
                if cached_leaderboard.value().valid_until > chrono::Utc::now() {
                    return Ok(web::Json(cached_leaderboard.board.to_owned()));
                } else {
                    drop(cached_leaderboard);
                    cache.remove(&lid);
                }
            }
            let mut conn = db.get()?;
            let board = block(move || database::get_leaderboard(&mut conn, &path.0)).await??;
            cache.insert(
                lid,
                CachedLeaderboard {
                    board: board.clone(),
                    valid_until: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            );

            Ok(web::Json(board))
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
    let mut conn = db.get()?;
    let name = path.0.clone();
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &name)).await? {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await?? {
            let mut conn = db.get()?;
            block(move || database::delete_leaderboard(&mut conn, &path.0)).await??;
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
            &mut db.get()?,
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
                TimeError::DieselError(diesel::result::Error::NotFound) => {
                    TimeError::LeaderboardNotFound
                }
                _ => e,
            })
        }
        Ok(leaderboard) => Ok(web::Json(json!(leaderboard))),
    }
}

#[post("/leaderboards/{name}/leave")]
pub async fn leave_leaderboard(
    user: UserId,
    path: Path<(String,)>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &path.0)).await?
    {
        let mut conn = db.get()?;
        let mut conn2 = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await??
            && block(move || database::get_leaderboard_admin_count(&mut conn2, lid)).await?? == 1
        {
            return Err(TimeError::LastAdmin);
        }
        let left =
            block(move || database::remove_user_from_leaderboard(&mut db.get()?, lid, user.id))
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
    let mut conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &path.0)).await?
    {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(newadmin) =
                block(move || database::get_user_by_name(&mut conn, &promotion.user)).await?
            {
                let mut conn = db.get()?;
                if block(move || {
                    database::promote_user_to_leaderboard_admin(&mut conn, lid, newadmin.id)
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
    let mut conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &path.0)).await?
    {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(oldadmin) =
                block(move || database::get_user_by_name(&mut conn, &demotion.user)).await?
            {
                let mut conn = db.get()?;
                if block(move || {
                    database::demote_user_to_leaderboard_member(&mut conn, lid, oldadmin.id)
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
    let mut conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &path.0)).await?
    {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(kmember) =
                block(move || database::get_user_by_name(&mut conn, &kick.user)).await?
            {
                let mut conn = db.get()?;
                if block(move || database::remove_user_from_leaderboard(&mut conn, lid, kmember.id))
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
    let mut conn = db.get()?;
    if let Ok(lid) = block(move || database::get_leaderboard_id_by_name(&mut conn, &path.0)).await?
    {
        let mut conn = db.get()?;
        if block(move || database::is_leaderboard_admin(&mut conn, user.id, lid)).await?? {
            let mut conn = db.get()?;
            let code =
                block(move || database::regenerate_leaderboard_invite(&mut conn, lid)).await??;
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
