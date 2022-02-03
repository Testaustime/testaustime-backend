use std::{future::Future, pin::Pin, sync::Arc};

use actix_web::{
    dev::Payload,
    error::*,
    http::header::ContentType,
    web::{Data, Json},
    Error, FromRequest, HttpRequest, HttpResponse, Responder,
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope,
    TokenResponse,
};
use serde_derive::{Deserialize, Serialize};

use crate::{database::Database, user::User};

impl FromRequest for User {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<User, Error>>>>;

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        let db = Data::extract(req);
        let headers = req.headers().clone();
        Box::pin(async move {
            let db: Data<Database> = db.await.unwrap();
            if let Some(auth) = headers.get("Authorization") {
                let auth = auth.to_str().unwrap();
                if let Some(token) = auth.trim().strip_prefix("Bearer ") {
                    if let Ok(user) = db.get_user_by_token(token) {
                        Ok(user)
                    } else {
                        Err(ErrorUnauthorized("Unauthorized"))
                    }
                } else {
                    Err(ErrorUnauthorized("unauthorized"))
                }
            } else {
                Err(ErrorUnauthorized("unauthorized"))
            }
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartBeat {
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}

#[post("/activity/update")]
pub async fn activity(
    user: User,
    heartbeat: Json<HeartBeat>,
    db: Data<Database>,
) -> impl Responder {
    db.update_activity(user.0, heartbeat.into_inner()).unwrap();
    HttpResponse::Ok()
}
