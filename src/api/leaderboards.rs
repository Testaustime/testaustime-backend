use axum::{Json, extract::Path};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    database::DatabaseWrapper,
    error::TimeError,
    models::{PrivateLeaderboard, UserId, UserIdentity},
};

use super::users::MinimalLeaderboard;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct LeaderboardCreateRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LeaderboardUser {
    pub user: String,
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardCreateResponse {
    invite_code: String,
}

#[utoipa::path(
    post,
    path = "/leaderboards/create",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = LeaderboardCreateResponse)
    )
)]
pub async fn create_leaderboard(
    creator: UserId,
    db: DatabaseWrapper,
    body: Json<LeaderboardCreateRequest>,
) -> Result<Json<LeaderboardCreateResponse>, TimeError> {
    if !super::VALID_NAME_REGEX.is_match(&body.name) {
        return Err(TimeError::BadLeaderboardName);
    }

    if db.get_leaderboard_id_by_name(&body.name).await.is_ok() {
        return Err(TimeError::LeaderboardExists);
    }

    match db.create_leaderboard(creator.id, &body.name).await {
        Ok(code) => Ok(Json(LeaderboardCreateResponse { invite_code: code })),
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(DieselError::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::LeaderboardExists,
                _ => e,
            })
        }
    }
}

#[utoipa::path(
    get,
    path = "/leaderboards/{name}",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = PrivateLeaderboard)
    )
)]
pub async fn get_leaderboard(
    user: UserId,
    Path(name): Path<String>,
    db: DatabaseWrapper,
) -> Result<Json<PrivateLeaderboard>, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_member(user.id, lid).await? {
        let board = db.get_leaderboard(&name).await?;
        Ok(Json(board))
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[utoipa::path(
    delete,
    path = "/leaderboards/{name}",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn delete_leaderboard(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
) -> Result<StatusCode, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await? {
        db.delete_leaderboard(&name).await?;
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct LeaderboardInvite {
    pub invite: String,
}

#[utoipa::path(
    post,
    path = "/leaderboards/join",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = MinimalLeaderboard)
    )
)]
pub async fn join_leaderboard(
    user: UserId,
    db: DatabaseWrapper,
    body: Json<LeaderboardInvite>,
) -> Result<Json<MinimalLeaderboard>, TimeError> {
    match db
        .add_user_to_leaderboard(user.id, body.invite.trim().trim_start_matches("ttlic_"))
        .await
    {
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(DieselError::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::AlreadyMember,
                TimeError::DieselError(DieselError::NotFound) => TimeError::LeaderboardNotFound,
                _ => e,
            })
        }
        Ok(leaderboard) => Ok(Json(leaderboard)),
    }
}

#[utoipa::path(
    post,
    path = "/leaderboards/{name}/leave",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = []),
    ),
    responses(
        (status = OK)
    )
)]
pub async fn leave_leaderboard(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
) -> Result<StatusCode, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await?
        && db.get_leaderboard_admin_count(lid).await? == 1
    {
        return Err(TimeError::LastAdmin);
    }

    if db.remove_user_from_leaderboard(lid, user.id).await? {
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::NotMember)
    }
}

#[utoipa::path(
    post,
    path = "/leaderboards/{name}/promote",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn promote_member(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
    promotion: Json<LeaderboardUser>,
) -> Result<StatusCode, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await? {
        let newadmin = db
            .get_user_by_name(&promotion.user)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if db
            .promote_user_to_leaderboard_admin(lid, newadmin.id)
            .await?
        {
            Ok(StatusCode::OK)
        } else {
            // FIXME: This is not correct
            Err(TimeError::NotMember)
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[utoipa::path(
    post,
    path = "/leaderboards/{name}/demote",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn demote_member(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
    demotion: Json<LeaderboardUser>,
) -> Result<StatusCode, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await? {
        let oldadmin = db
            .get_user_by_name(&demotion.user)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if db
            .demote_user_to_leaderboard_member(lid, oldadmin.id)
            .await?
        {
            Ok(StatusCode::OK)
        } else {
            // FIXME: This is not correct
            Err(TimeError::NotMember)
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[utoipa::path(
    post,
    path = "/leaderboards/{name}/kick",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn kick_member(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
    kick: Json<LeaderboardUser>,
) -> Result<StatusCode, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await? {
        let kmember = db
            .get_user_by_name(&kick.user)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        db.remove_user_from_leaderboard(lid, kmember.id)
            .await
            .map_err(|_| TimeError::NotMember)?;
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InviteCodeRegenerateResponse {
    invite_code: String,
}

#[utoipa::path(
    post,
    path = "/leaderboards/{name}/regenerate",
    params(
        ("name", description = "Leaderboard name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = InviteCodeRegenerateResponse)
    )
)]
pub async fn regenerate_invite(
    user: UserIdentity,
    Path(name): Path<String>,
    db: DatabaseWrapper,
) -> Result<Json<InviteCodeRegenerateResponse>, TimeError> {
    let lid = db
        .get_leaderboard_id_by_name(&name)
        .await
        .map_err(|_| TimeError::LeaderboardNotFound)?;

    if db.is_leaderboard_admin(user.id, lid).await? {
        let code = db.regenerate_leaderboard_invite(lid).await?;
        Ok(Json(InviteCodeRegenerateResponse { invite_code: code }))
    } else {
        Err(TimeError::Unauthorized)
    }
}
