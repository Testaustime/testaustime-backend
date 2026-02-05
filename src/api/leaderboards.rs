use axum::{extract::Path, Json};
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

/// Request body for creating a leaderboard.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LeaderboardCreateRequest {
    /// Name of the leaderboard (2-32 alphanumeric characters).
    pub name: String,
}

/// Request body specifying a leaderboard member.
#[derive(Deserialize, ToSchema)]
pub struct LeaderboardUser {
    /// Username of the target member.
    pub user: String,
}

/// Response from leaderboard creation.
#[derive(Serialize, ToSchema)]
pub struct LeaderboardCreateResponse {
    /// The invite code for others to join (starts with ttlic_).
    invite_code: String,
}

/// Create a new leaderboard.
///
/// Creates a leaderboard with the authenticated user as admin. Returns an invite code for others to join.
#[utoipa::path(
    post,
    path = "/leaderboards/create",
    request_body = LeaderboardCreateRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Leaderboard created", body = LeaderboardCreateResponse),
        (status = 400, description = "Invalid leaderboard name"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Leaderboard name already exists"),
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

/// Get leaderboard details and members.
///
/// Returns the leaderboard with all members and their coding times. Only accessible to members.
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
        (status = OK, description = "Leaderboard retrieved", body = PrivateLeaderboard),
        (status = 401, description = "Unauthorized or not a member"),
        (status = 404, description = "Leaderboard not found"),
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

/// Delete a leaderboard.
///
/// Permanently deletes the leaderboard. Only accessible to admins.
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
        (status = OK, description = "Leaderboard deleted"),
        (status = 401, description = "Unauthorized or not an admin"),
        (status = 404, description = "Leaderboard not found"),
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

/// Request body for joining a leaderboard.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LeaderboardInvite {
    /// The invite code (starts with ttlic_).
    pub invite: String,
}

/// Join a leaderboard with invite code.
///
/// Invite codes start with `ttlic_`. Returns basic leaderboard info on success.
#[utoipa::path(
    post,
    path = "/leaderboards/join",
    request_body = LeaderboardInvite,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Joined leaderboard", body = MinimalLeaderboard),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invalid invite code"),
        (status = 409, description = "Already a member"),
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

/// Leave a leaderboard.
///
/// Removes the user from the leaderboard. Admins cannot leave if they are the last admin.
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
        (status = OK, description = "Left leaderboard"),
        (status = 400, description = "Cannot leave as last admin"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Leaderboard not found or not a member"),
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

/// Promote a member to admin.
///
/// Only accessible to existing admins. The promoted user gains admin privileges.
#[utoipa::path(
    post,
    path = "/leaderboards/{name}/promote",
    params(
        ("name", description = "Leaderboard name")
    ),
    request_body = LeaderboardUser,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Member promoted to admin"),
        (status = 401, description = "Unauthorized or not an admin"),
        (status = 404, description = "Leaderboard or user not found"),
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

/// Demote an admin to member.
///
/// Only accessible to existing admins. The demoted user loses admin privileges.
#[utoipa::path(
    post,
    path = "/leaderboards/{name}/demote",
    params(
        ("name", description = "Leaderboard name")
    ),
    request_body = LeaderboardUser,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Admin demoted to member"),
        (status = 401, description = "Unauthorized or not an admin"),
        (status = 404, description = "Leaderboard or user not found"),
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

/// Remove a member from leaderboard.
///
/// Only accessible to admins. Removes the specified user from the leaderboard.
#[utoipa::path(
    post,
    path = "/leaderboards/{name}/kick",
    params(
        ("name", description = "Leaderboard name")
    ),
    request_body = LeaderboardUser,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Member removed from leaderboard"),
        (status = 401, description = "Unauthorized or not an admin"),
        (status = 404, description = "Leaderboard or user not found"),
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

/// Response containing the newly generated invite code.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InviteCodeRegenerateResponse {
    /// The new invite code (starts with ttlic_).
    invite_code: String,
}

/// Generate new leaderboard invite code.
///
/// Only accessible to admins. Invalidates the previous invite code.
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
        (status = OK, description = "New invite code generated", body = InviteCodeRegenerateResponse),
        (status = 401, description = "Unauthorized or not an admin"),
        (status = 404, description = "Leaderboard not found"),
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
