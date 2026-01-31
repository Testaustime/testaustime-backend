use std::sync::Arc;

use axum::{Json, extract::State};
use diesel::result::DatabaseErrorKind;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    api::activity::HeartBeatMemoryStore,
    database::DatabaseWrapper,
    error::TimeError,
    models::{CurrentActivity, FriendWithTimeAndStatus, UserId, UserIdentity},
};

/// Request body for adding a friend.
#[derive(Deserialize, Debug, ToSchema)]
pub struct FriendRequest {
    /// The friend code (starts with ttfc_).
    pub code: String,
}

/// Add a friend using their friend code.
///
/// Friend codes start with `ttfc_`. Returns the friend's profile with coding stats.
#[utoipa::path(
    post,
    path = "/friends/add",
    request_body = FriendRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Friend added successfully", body = FriendWithTimeAndStatus),
        (status = 400, description = "Invalid friend code"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Already friends"),
    )
)]
pub async fn add_friend(
    user: UserId,
    db: DatabaseWrapper,
    State(heartbeats): State<Arc<HeartBeatMemoryStore>>,
    Json(body): Json<FriendRequest>,
) -> Result<Json<FriendWithTimeAndStatus>, TimeError> {
    match db
        .add_friend(user.id, body.code.trim_start_matches("ttfc_").to_string())
        .await
    {
        // This is not correct
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::AlreadyFriends,
                _ => e,
            })
        }
        Ok(friend) => {
            let friend_with_time = FriendWithTimeAndStatus {
                username: friend.username.clone(),
                coding_time: db.get_coding_time_steps(friend.id).await,
                status: heartbeats.get(&friend.id).map(|heartbeat| {
                    let (mut inner_heartbeat, start_time, duration) = heartbeat.to_owned();
                    drop(heartbeat);
                    if inner_heartbeat.hidden == Some(true) {
                        inner_heartbeat.project_name = Some(String::from("hidden"));
                    }
                    CurrentActivity {
                        started: start_time,
                        duration: duration.num_seconds(),
                        heartbeat: inner_heartbeat,
                    }
                }),
            };

            Ok(Json(friend_with_time))
        }
    }
}

/// Get list of friends with coding stats.
///
/// Returns all friends with their coding time and current activity status.
#[utoipa::path(
    get,
    path = "/friends/list",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Friends list retrieved", body = Vec<FriendWithTimeAndStatus>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_friends(
    user: UserId,
    db: DatabaseWrapper,
    State(heartbeats): State<Arc<HeartBeatMemoryStore>>,
) -> Result<Json<Vec<FriendWithTimeAndStatus>>, TimeError> {
    let friends = db
        .get_friends_with_time(user.id)
        .await
        .inspect_err(|e| error!("{e}"))?
        .into_iter()
        .map(|fwt| FriendWithTimeAndStatus {
            username: fwt.user.username,
            coding_time: fwt.coding_time,
            status: heartbeats.get(&fwt.user.id).map(|heartbeat| {
                let (mut inner_heartbeat, start_time, duration) = heartbeat.to_owned();
                drop(heartbeat);
                if inner_heartbeat.hidden == Some(true) {
                    inner_heartbeat.project_name = Some(String::from("hidden"));
                }
                CurrentActivity {
                    started: start_time,
                    duration: duration.num_seconds(),
                    heartbeat: inner_heartbeat,
                }
            }),
        })
        .collect::<Vec<_>>();

    Ok(Json(friends))
}

/// Response containing the newly generated friend code.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegenerateFriendCodeResponse {
    /// The new friend code (starts with ttfc_).
    friend_code: String,
}

/// Generate a new friend code.
///
/// Invalidates the previous friend code. Others must use the new code to add you as a friend.
#[utoipa::path(
    post,
    path = "/friends/regenerate",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "New friend code generated", body = RegenerateFriendCodeResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn regenerate_friend_code(
    user: UserIdentity,
    db: DatabaseWrapper,
) -> Result<Json<RegenerateFriendCodeResponse>, TimeError> {
    db.regenerate_friend_code(user.id)
        .await
        .inspect_err(|e| error!("{}", e))
        .map(|c| Json(RegenerateFriendCodeResponse { friend_code: c }))
}

/// Request body for removing a friend.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveFriendRequest {
    /// Username of the friend to remove.
    name: String,
}

/// Remove a friend.
///
/// Removes the friendship in both directions.
#[utoipa::path(
    delete,
    path = "/friends/remove",
    request_body = RemoveFriendRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Friend removed"),
        (status = 400, description = "Friend not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn remove(
    user: UserIdentity,
    db: DatabaseWrapper,
    Json(body): Json<RemoveFriendRequest>,
) -> Result<StatusCode, TimeError> {
    let friend = db.get_user_by_name(&body.name).await?;
    let deleted = db.remove_friend(user.id, friend.id).await?;

    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::BadId)
    }
}
