use actix_web::{
    error::*,
    web::{self, block, Data, Json, Path, Query},
    HttpResponse, Responder,
};
use chrono::{Duration, Local};
use dashmap::DashMap;

use crate::{database::Database, requests::*, user::UserId};

pub type HeartBeatMemoryStore =
    DashMap<UserId, (HeartBeat, chrono::NaiveDateTime, chrono::Duration)>;

#[post("/activity/update")]
pub async fn update(
    user: UserId,
    heartbeat: Json<HeartBeat>,
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder> {
    let heartbeat = heartbeat;
    match heartbeats.get(&user) {
        Some(test) => {
            let (inner_heartbeat, start, duration) = test.to_owned();
            drop(test);
            let (start, duration) = (start, duration);
            let curtime = Local::now().naive_local();
            if heartbeat.eq(&inner_heartbeat) {
                if curtime.signed_duration_since(start + duration) > Duration::seconds(900) {
                    // If the user sends a heartbeat but maximum activity duration has been exceeded,
                    // end session and start new
                    let res =
                        match db.add_activity(user.id, inner_heartbeat.clone(), start, duration) {
                            Ok(_) => Ok(HttpResponse::Ok().finish()),
                            Err(e) => Err(ErrorInternalServerError(e)),
                        };
                    heartbeats.insert(
                        user,
                        (
                            heartbeat.into_inner(),
                            Local::now().naive_local(),
                            Duration::seconds(0),
                        ),
                    );
                    res
                } else {
                    // Extend current coding session if heartbeat matches and it has been under the maximum duration of a break
                    heartbeats.insert(
                        user,
                        (
                            heartbeat.into_inner(),
                            start,
                            curtime.signed_duration_since(start),
                        ),
                    );
                    Ok(HttpResponse::Ok().finish())
                }
            } else {
                // Flush current session and start new session if heartbeat changes
                let res = match db.add_activity(user.id, inner_heartbeat.clone(), start, duration) {
                    Ok(_) => Ok(HttpResponse::Ok().finish()),
                    Err(e) => Err(ErrorInternalServerError(e)),
                };
                heartbeats.insert(
                    user,
                    (
                        heartbeat.into_inner(),
                        Local::now().naive_local(),
                        Duration::seconds(0),
                    ),
                );
                res
            }
        }
        None => {
            // If the user has not sent a heartbeat during this session
            heartbeats.insert(
                user,
                (
                    heartbeat.into_inner(),
                    Local::now().naive_local(),
                    Duration::seconds(0),
                ),
            );
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[post("/activity/flush")]
pub async fn flush(
    user: UserId,
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> Result<impl Responder> {
    match heartbeats.get(&user) {
        Some(flushme) => {
            let (inner_heartbeat, start, duration) = flushme.to_owned();
            drop(flushme);
            heartbeats.remove(&user);
            match db.add_activity(user.id, inner_heartbeat, start, duration) {
                Ok(_) => Ok(HttpResponse::Ok().finish()),
                Err(e) => Err(ErrorInternalServerError(e)),
            }
        }
        None => Ok(HttpResponse::Ok().finish()),
    }
}

#[get("/users/{username}/activity/data")]
pub async fn get_activities(
    data: Query<DataRequest>,
    path: Path<(String,)>,
    user: UserId,
    db: Data<Database>,
) -> Result<impl Responder> {
    if path.0 == "@me" {
        let data = block(move || db.get_activity(data.into_inner(), user.id).unwrap()).await?;
        Ok(web::Json(data))
    } else {
        let db_clone = db.clone();
        let friend_id = db_clone.get_user_by_name(&path.0)?.id;
        match db.are_friends(user.id, friend_id) {
            Ok(b) => {
                if b {
                    let data =
                        block(move || db.get_activity(data.into_inner(), friend_id).unwrap())
                            .await?;
                    Ok(web::Json(data))
                } else {
                    Err(ErrorUnauthorized("This user is not your friend"))
                }
            }
            Err(e) => Err(ErrorInternalServerError(e)),
        }
    }
}
