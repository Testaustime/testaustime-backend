use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{self, Data, Json, Query},
    Error, FromRequest, HttpRequest, HttpResponse, Responder,
};
use chrono::{serde::ts_seconds_option, DateTime, Duration, Local, Utc};
use dashmap::DashMap;
use serde_derive::{Deserialize, Serialize};

use crate::{database::Database, user::UserId};

#[derive(Deserialize, Debug)]
pub struct DataRequest {
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub to: Option<DateTime<Utc>>,
    pub min_duration: Option<i32>,
    pub editor_name: Option<String>,
    pub language: Option<String>,
    pub hostname: Option<String>,
    pub project_name: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
}

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Clone)]
pub struct HeartBeat {
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}

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
                        match db.add_activity(user.0, inner_heartbeat.clone(), start, duration) {
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
                let res = match db.add_activity(user.0, inner_heartbeat.clone(), start, duration) {
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

// TODO: Implement requests for getting the data of users other than the current user
#[get("/activity/data")]
pub async fn get_activities(
    data: Query<DataRequest>,
    user: UserId,
    db: Data<Database>,
) -> impl Responder {
    let data = db.get_activity(data.into_inner(), user).unwrap();
    web::Json(data)
}

pub async fn register(data: Json<RegisterRequest>, db: Data<Database>) -> impl Responder {
    match db.new_user(&data.username) {
        Ok(user) => HttpResponse::Ok().body(user),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}
