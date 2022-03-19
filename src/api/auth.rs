use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{Data, Json},
    Error, FromRequest, HttpRequest, HttpResponse, Responder,
};

use crate::{database::Database, requests::*, user::UserId};

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
                Ok(UserId { id: user })
            } else {
                Err(ErrorUnauthorized("Unauthorized"))
            }
        })
    }
}

#[post("/auth/login")]
pub async fn login(data: Json<RegisterRequest>, db: Data<Database>) -> Result<impl Responder> {
    match db.get_user_by_name(&data.username) {
        Ok(user) => match db.verify_user_password(&data.username, &data.password) {
            Ok(true) => {
                let token = json!({ "token": user.auth_token }).to_string();
                Ok(HttpResponse::Ok().body(token))
            }
            Ok(false) => Ok(HttpResponse::Unauthorized().body("Invalid password or username")),
            Err(e) => Err(ErrorInternalServerError(e)),
        },
        Err(_) => Ok(HttpResponse::Unauthorized().body("No such user")),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<Database>) -> Result<impl Responder> {
    match db.regenerate_token(user.id) {
        Ok(token) => {
            let token = json!({ "token": token }).to_string();
            Ok(HttpResponse::Ok().body(token))
        }
        Err(e) => {
            error!("{}", e);
            Err(ErrorInternalServerError(e))
        }
    }
}

#[post("/auth/register")]
pub async fn register(data: Json<RegisterRequest>, db: Data<Database>) -> Result<impl Responder> {
    if data.password.len() < 8 {
        return Err(actix_web::error::ErrorBadRequest(
            "Password has to be at least 8 characters long",
        ));
    }
    match db.new_user(&data.username, &data.password) {
        Ok(token) => {
            let token = json!({ "token": token }).to_string();
            Ok(HttpResponse::Ok().body(token))
        }
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[post("/auth/changepassword")]
pub async fn changepassword(
    userid: UserId,
    data: Json<PasswordChangeRequest>,
    db: Data<Database>,
) -> Result<impl Responder> {
    if data.new.len() < 8 {
        return Err(actix_web::error::ErrorBadRequest(
            "Password has to be at least 8 characters long",
        ));
    }
    match db.get_user_by_id(userid.id) {
        Ok(user) => {
            match db.verify_user_password(&user.user_name, &data.old) {
                Ok(k) => {
                    if k || user.password.iter().all(|n| *n == 0) {
                        // Some noobs don't have password (me)
                        match db.change_user_password_to(userid.id, &data.new) {
                            Ok(_) => Ok(HttpResponse::Ok().finish()),
                            Err(e) => Err(ErrorInternalServerError(e)),
                        }
                    } else {
                        Err(ErrorUnauthorized("Invalid password or username"))
                    }
                }
                Err(e) => Err(ErrorInternalServerError(e)),
            }
        }
        Err(e) => {
            error!("{}", e);
            Err(ErrorInternalServerError(e))
        }
    }
}
