use std::{future::Future, ops::Add, pin::Pin};

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
    let heartbeat = heartbeat.into_inner();
    match heartbeats.get(&user) {
        Some(test) => {
            let (inner_heartbeat, start, duration) = test.clone();
            drop(test);
            let (start, duration) = (start, duration);
            let curtime = Local::now().naive_local();
            if heartbeat.eq(&inner_heartbeat) {
                if curtime.signed_duration_since(start + duration) > Duration::seconds(900) {
                    let res = match db.add_activity(user.0, heartbeat.clone(), start, duration) {
                        Ok(_) => HttpResponse::Ok().finish(),
                        Err(_) => HttpResponse::InternalServerError().finish(),
                    };
                    heartbeats.insert(
                        user,
                        (heartbeat, Local::now().naive_local(), Duration::seconds(0)),
                    );
                    res
                } else {
                    heartbeats.insert(
                        user,
                        (heartbeat, start, curtime.signed_duration_since(start)),
                    );
                    HttpResponse::Ok().finish()
                }
            } else {
                heartbeats.insert(
                    user,
                    (heartbeat, start, curtime.signed_duration_since(start)),
                );
                HttpResponse::Ok().finish()
            }
        }
        None => {
            heartbeats.insert(
                user,
                (heartbeat, Local::now().naive_local(), Duration::seconds(0)),
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
