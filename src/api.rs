use std::sync::Arc;

use actix_web::{
    FromRequest,
    dev::Payload,
    error::*,
    Error,
    http::header::ContentType,
    web::{Data, Json},
    HttpRequest, HttpResponse, Responder,
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope,
    TokenResponse,
};
use serde_derive::{Deserialize, Serialize};

use crate::database::User;

use std::future::Future;
use std::pin::Pin;

use crate::database::Database;


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
                    if let Ok(user) = db.get_user_by_token(token).await {
                        Ok(user)
                    } else {
                        Err(ErrorUnauthorized("Unauthorized"))
                    }
                } else {
                    Err(ErrorUnauthorized("unauthorized"))
                }
            } else {
                Err(ErrorUnauthorized("unauthorized"))
            };
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartBeat {
    project_name: Option<String>,
    language: Option<String>,
    editor_name: Option<String>,
    hostname: Option<String>,
}

#[post("/update_activity")]
pub async fn activity(user: User, heartbeat: Json<HeartBeat>, db: Data<Database>) -> impl Responder {
    format!("{:?} sent {:?}", user, heartbeat.project_name)
}
