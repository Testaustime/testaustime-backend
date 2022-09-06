use actix_web::{
    error::*,
    web::{self, block, Data, Json, Path},
    HttpResponse, Responder,
};
use dashmap::DashMap;
use diesel::result::DatabaseErrorKind;
use serde::Deserialize;

use crate::{
    database::DatabaseConnection,
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
    if block(move || conn.get_leaderboard_id_by_name(&lname))
        .await?
        .is_ok()
    {
        return Err(TimeError::LeaderboardExists);
    }

    match block(move || db.get()?.create_leaderboard(creator.id, &body.name)).await? {
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&name)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_member(user.id, lid)).await?? {
            if let Some(cached_leaderboard) = cache.get(&lid) {
                if cached_leaderboard.value().valid_until > chrono::Utc::now() {
                    return Ok(web::Json(cached_leaderboard.board.to_owned()));
                } else {
                    drop(cached_leaderboard);
                    cache.remove(&lid);
                }
            }
            let mut conn = db.get()?;
            let board = block(move || conn.get_leaderboard(&path.0)).await??;
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&name)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await?? {
            let mut conn = db.get()?;
            block(move || conn.delete_leaderboard(&path.0)).await??;
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
        db.get()?
            .add_user_to_leaderboard(user.id, body.invite.trim().trim_start_matches("ttlic_"))
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&path.0)).await? {
        let mut conn = db.get()?;
        let mut conn2 = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await??
            && block(move || conn2.get_leaderboard_admin_count(lid)).await?? == 1
        {
            return Err(TimeError::LastAdmin);
        }
        let left = block(move || db.get()?.remove_user_from_leaderboard(lid, user.id)).await??;
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&path.0)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(newadmin) = block(move || conn.get_user_by_name(&promotion.user)).await? {
                if block(move || {
                    db.get()?
                        .promote_user_to_leaderboard_admin(lid, newadmin.id)
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&path.0)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(oldadmin) = block(move || conn.get_user_by_name(&demotion.user)).await? {
                if block(move || {
                    db.get()?
                        .demote_user_to_leaderboard_member(lid, oldadmin.id)
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&path.0)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await?? {
            let mut conn = db.get()?;
            if let Ok(kmember) = block(move || conn.get_user_by_name(&kick.user)).await? {
                if block(move || db.get()?.remove_user_from_leaderboard(lid, kmember.id)).await?? {
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
    if let Ok(lid) = block(move || conn.get_leaderboard_id_by_name(&path.0)).await? {
        let mut conn = db.get()?;
        if block(move || conn.is_leaderboard_admin(user.id, lid)).await?? {
            let code = block(move || db.get()?.regenerate_leaderboard_invite(lid)).await??;
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
