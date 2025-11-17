use std::sync::Arc;

use axum::{extract::State, Json};
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

#[derive(Deserialize, Debug, ToSchema)]
pub struct FriendRequest {
    pub code: String,
}

#[utoipa::path(
    post,
    path = "/friends/add",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = FriendWithTimeAndStatus)
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

#[utoipa::path(
    get,
    path = "/friends/list",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = Vec<FriendWithTimeAndStatus>)
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegenerateFriendCodeResponse {
    friend_code: String,
}

#[utoipa::path(
    post,
    path = "/friends/regenerate",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = RegenerateFriendCodeResponse)
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveFriendRequest {
    name: String,
}

#[utoipa::path(
    delete,
    path = "/friends/remove",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
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
