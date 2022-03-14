use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{self, Data, Json, Path, Query},
    Error, FromRequest, HttpRequest, HttpResponse, Responder,
};
use chrono::{Duration, Local};
use dashmap::DashMap;

use crate::{database::Database, requests::*, user::UserId};

pub type HeartBeatMemoryStore =
    DashMap<UserId, (HeartBeat, chrono::NaiveDateTime, chrono::Duration)>;

impl FromRequest for UserId {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<UserId, Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let db = Data::extract(req);
        let headers = req.headers().clone();
        Box::pin(async move {
            let db: Data<Database> = db.await?;
            let Some(auth) = headers.get("Authorization") else { return Err(ErrorUnauthorized("Unauthorized")) };
            let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ") else { return Err(ErrorUnauthorized("Unathorized")) };
            if let Ok(user) = db.get_user_by_token(token) {
                Ok(user)
            } else {
                Err(ErrorUnauthorized("Unauthorized"))
            }
        })
    }
}

#[post("/activity/update")]
pub async fn update(
    user: UserId,
    heartbeat: Json<HeartBeat>,
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> impl Responder {
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
                            Ok(_) => HttpResponse::Ok().finish(),
                            Err(_) => HttpResponse::InternalServerError().finish(),
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
                    HttpResponse::Ok().finish()
                }
            } else {
                // Flush current session and start new session if heartbeat changes
                let res = match db.add_activity(user.id, inner_heartbeat.clone(), start, duration) {
                    Ok(_) => HttpResponse::Ok().finish(),
                    Err(_) => HttpResponse::InternalServerError().finish(),
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
            HttpResponse::Ok().finish()
        }
    }
}

#[post("/activity/flush")]
pub async fn flush(
    user: UserId,
    db: Data<Database>,
    heartbeats: Data<HeartBeatMemoryStore>,
) -> impl Responder {
    match heartbeats.get(&user) {
        Some(flushme) => {
            let (inner_heartbeat, start, duration) = flushme.to_owned();
            drop(flushme);
            heartbeats.remove(&user);
            match db.add_activity(user.id, inner_heartbeat, start, duration) {
                Ok(_) => HttpResponse::Ok().finish(),
                Err(_) => HttpResponse::InternalServerError().finish(),
            }
        }
        None => HttpResponse::Ok().finish(),
    }
}

// TODO: Implement requests for getting the data of users other than the current user
#[get("/users/{username}/data")]
pub async fn get_activities(
    data: Query<DataRequest>,
    path: Path<(String,)>,
    user: UserId,
    db: Data<Database>,
) -> Result<impl Responder> {
    if path.0 == "@me" {
        let data = db.get_activity(data.into_inner(), user.id).unwrap();
        Ok(web::Json(data))
    } else {
        let friend_id = db.get_user_by_name(&path.0).unwrap().id;
        match db.are_friends(user.id, friend_id) {
            Ok(b) => {
                if b {
                    let data = db.get_activity(data.into_inner(), friend_id).unwrap();
                    Ok(web::Json(data))
                } else {
                    Err(actix_web::error::ErrorUnauthorized(
                        "This user is not your friend",
                    ))
                }
            }
            Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
        }
    }
}

#[post("/auth/register")]
pub async fn register(data: Json<RegisterRequest>, db: Data<Database>) -> impl Responder {
    match db.new_user(&data.username, &data.password) {
        Ok(token) => HttpResponse::Ok().body(token),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

#[post("/auth/login")]
pub async fn login(data: Json<RegisterRequest>, db: Data<Database>) -> impl Responder {
    match db.get_user_by_name(&data.username) {
        Ok(user) => match db.verify_user_password(&data.username, &data.password) {
            Ok(true) => HttpResponse::Ok().body(user.auth_token),
            Ok(false) => HttpResponse::Unauthorized().body("Invalid password or username"),
            Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
        },
        Err(_) => HttpResponse::Unauthorized().body("No such user"),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.regenerate_token(user) {
        Ok(token) => Ok(HttpResponse::Ok().body(token)),
        Err(e) => {
            error!("{}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

#[post("/friends/add")]
pub async fn add_friend(user: UserId, body: String, db: Data<Database>) -> Result<impl Responder> {
    if let Err(e) = db.add_friend(user, &body.trim().trim_start_matches("ttfc_")) {
        // This is not correct
        error!("{}", e);
        Err(actix_web::error::ErrorInternalServerError(e))
    } else {
        Ok(HttpResponse::Ok().finish())
    }
}

#[get("/friends/list")]
pub async fn get_friends(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.get_friends(user) {
        Ok(friends) => Ok(web::Json(friends)),
        Err(e) => {
            error!("{}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}
