use actix_web::{
    error::*,
    web::{self, Data},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;

use crate::{
    api::{activity::HeartBeatMemoryStore, auth::SecuredUserIdentity},
    database::DatabaseWrapper,
    error::TimeError,
    models::{CurrentActivity, FriendWithTimeAndStatus, UserId},
};

#[post("/friends/add")]
pub async fn add_friend(
    user: UserId,
    body: String,
    db: DatabaseWrapper,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder, TimeError> {
    match db
        .add_friend(user.id, body.trim().trim_start_matches("ttfc_").to_string())
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
                    let (inner_heartbeat, start_time, duration) = heartbeat.to_owned();
                    drop(heartbeat);
                    CurrentActivity {
                        started: start_time,
                        duration: duration.num_seconds(),
                        heartbeat: inner_heartbeat,
                    }
                }),
            };

            Ok(web::Json(friend_with_time))
        }
    }
}

#[get("/friends/list")]
pub async fn get_friends(
    user: UserId,
    db: DatabaseWrapper,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder, TimeError> {
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
                    inner_heartbeat.project_name = Some("".to_string());
                }
                CurrentActivity {
                    started: start_time,
                    duration: duration.num_seconds(),
                    heartbeat: inner_heartbeat,
                }
            }),
        })
        .collect::<Vec<_>>();

    Ok(web::Json(friends))
}

#[post("/friends/regenerate")]
pub async fn regenerate_friend_code(
    user: SecuredUserIdentity,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    db.regenerate_friend_code(user.identity.id)
        .await
        .inspect_err(|e| error!("{}", e))
        .map(|code| web::Json(json!({ "friend_code": code })))
}

#[delete("/friends/remove")]
pub async fn remove(
    user: SecuredUserIdentity,
    db: DatabaseWrapper,
    body: String,
) -> Result<impl Responder, TimeError> {
    let friend = db.get_user_by_name(body.clone()).await?;
    let deleted = db.remove_friend(user.identity.id, friend.id).await?;

    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::BadId)
    }
}
