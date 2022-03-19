use actix_web::{
    error::*,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::{Duration, Local};
use dashmap::DashMap;

use crate::{database::Database, requests::*, user::UserId};

pub type HeartBeatMemoryStore =
    DashMap<UserId, (HeartBeat, chrono::NaiveDateTime, chrono::Duration)>;

#[post("/update")]
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
                            Ok(_) => Ok(HttpResponse::Ok().body(0.to_string())),
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
                    Ok(HttpResponse::Ok().body(curtime.signed_duration_since(start).to_string()))
                }
            } else {
                // Flush current session and start new session if heartbeat changes
                let res = match db.add_activity(user.id, inner_heartbeat.clone(), start, duration) {
                    Ok(_) => Ok(HttpResponse::Ok().body(0.to_string())),
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
            Ok(HttpResponse::Ok().body(0.to_string()))
        }
    }
}

#[post("/flush")]
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

#[delete("/activity/delete")]
pub async fn delete(user: UserId, db: Data<Database>, body: String) -> Result<impl Responder> {
    let deleted = db.delete_activity(
        user.id,
        body.parse::<i32>().map_err(|e| ErrorBadRequest(e))?,
    )?;
    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ErrorBadRequest("Invalid id or Unauthorized"))
    }
}
