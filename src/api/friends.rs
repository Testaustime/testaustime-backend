use actix_web::{
    error::*,
    web::{self, block, Data},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;

use crate::{
    api::activity::HeartBeatMemoryStore,
    database::Database,
    error::TimeError,
    models::{CodingTimeSteps, CurrentActivity, FriendWithTimeAndStatus, UserId},
};

#[post("/friends/add")]
pub async fn add_friend(
    user: UserId,
    body: String,
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    match block(move || conn.add_friend(user.id, body.trim().trim_start_matches("ttfc_"))).await? {
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
            let mut conn = db.get()?;
            let friend_with_time = FriendWithTimeAndStatus {
                username: friend.username.clone(),
                coding_time: match block(move || conn.get_coding_time_steps(friend.id)).await {
                    Ok(coding_time_steps) => coding_time_steps,
                    _ => CodingTimeSteps {
                        all_time: 0,
                        past_month: 0,
                        past_week: 0,
                    },
                },
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
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    match block(move || conn.get_friends(user.id)).await? {
        Ok(friends) => {
            let friends_with_time = futures::future::join_all(friends.iter().map(|friend| async {
                let mut conn2 = db.get().unwrap();
                let friend_id = friend.id;
                FriendWithTimeAndStatus {
                    username: friend.username.clone(),
                    coding_time: match block(move || conn2.get_coding_time_steps(friend_id)).await {
                        Ok(coding_time_steps) => coding_time_steps,
                        _ => CodingTimeSteps {
                            all_time: 0,
                            past_month: 0,
                            past_week: 0,
                        },
                    },
                    status: heartbeats.get(&friend_id).map(|heartbeat| {
                        let (inner_heartbeat, start_time, duration) = heartbeat.to_owned();
                        drop(heartbeat);
                        CurrentActivity {
                            started: start_time,
                            duration: duration.num_seconds(),
                            heartbeat: inner_heartbeat,
                        }
                    }),
                }
            }))
            .await;
            Ok(web::Json(friends_with_time))
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/friends/regenerate")]
pub async fn regenerate_friend_code(
    user: UserId,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    match block(move || db.get()?.regenerate_friend_code(user.id)).await? {
        Ok(code) => Ok(web::Json(json!({ "friend_code": code }))),
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[delete("/friends/remove")]
pub async fn remove(
    user: UserId,
    db: Data<Database>,
    body: String,
) -> Result<impl Responder, TimeError> {
    let mut conn = db.get()?;
    let friend = block(move || conn.get_user_by_name(&body)).await??;
    let deleted = block(move || db.get()?.remove_friend(user.id, friend.id)).await??;
    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::BadId)
    }
}
