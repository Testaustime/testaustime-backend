use actix_web::{
    error::*,
    web::{self, Json, Path},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;
use serde::{Deserialize, Serialize};

use crate::{
    api::auth::SecuredUserIdentity, database::DatabaseWrapper, error::TimeError, models::UserId,
};

#[derive(Deserialize, Serialize)]
pub struct LeaderboardName {
    pub name: String,
}

#[derive(Deserialize, Serialize)]
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
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    if !super::VALID_NAME_REGEX.is_match(&body.name) {
        return Err(TimeError::BadLeaderboardName);
    }
    let lname = body.name.clone();

    if db.get_leaderboard_id_by_name(lname).await.is_ok() {
        return Err(TimeError::LeaderboardExists);
    }

    match db.create_leaderboard(creator.id, &body.name).await {
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
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_member(user.id, lid).await? {
        let board = db.get_leaderboard(path.0.clone()).await?;
        Ok(web::Json(board))
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[delete("/leaderboards/{name}")]
pub async fn delete_leaderboard(
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await? {
        db.delete_leaderboard(path.0.clone()).await?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[post("/leaderboards/join")]
pub async fn join_leaderboard(
    user: UserId,
    body: Json<LeaderboardInvite>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    match db
        .add_user_to_leaderboard(
            user.id,
            body.invite.trim().trim_start_matches("ttlic_").to_string(),
        )
        .await
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
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await?
        && db.get_leaderboard_admin_count(lid).await? == 1
    {
        return Err(TimeError::LastAdmin);
    }

    if db
        .remove_user_from_leaderboard(lid, user.identity.id)
        .await?
    {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::NotMember)
    }
}

#[post("/leaderboards/{name}/promote")]
pub async fn promote_member(
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
    promotion: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await? {
        let newadmin = db
            .get_user_by_name(promotion.user.clone())
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if db
            .promote_user_to_leaderboard_admin(lid, newadmin.id)
            .await?
        {
            Ok(HttpResponse::Ok().finish())
        } else {
            // FIXME: This is not correct
            Err(TimeError::NotMember)
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[post("/leaderboards/{name}/demote")]
pub async fn demote_member(
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
    demotion: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await? {
        let oldadmin = db
            .get_user_by_name(demotion.user.clone())
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if db
            .demote_user_to_leaderboard_member(lid, oldadmin.id)
            .await?
        {
            Ok(HttpResponse::Ok().finish())
        } else {
            // FIXME: This is not correct
            Err(TimeError::NotMember)
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[post("/leaderboards/{name}/kick")]
pub async fn kick_member(
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
    kick: Json<LeaderboardUser>,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await? {
        let kmember = db
            .get_user_by_name(kick.user.clone())
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        db.remove_user_from_leaderboard(lid, kmember.id)
            .await
            .map_err(|_| TimeError::NotMember)?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[post("/leaderboards/{name}/regenerate")]
pub async fn regenerate_invite(
    user: SecuredUserIdentity,
    path: Path<(String,)>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(path.0.clone())
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.identity.id, lid).await? {
        let code = db.regenerate_leaderboard_invite(lid).await?;
        Ok(web::Json(json!({ "invite_code": code })))
    } else {
        Err(TimeError::Unauthorized)
    }
}
