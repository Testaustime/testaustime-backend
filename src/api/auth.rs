use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{block, Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder,
};

use crate::{
    database::Database, error::TimeError, models::RegisteredUser, requests::*, user::UserId,
};

impl FromRequest for UserId {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<UserId, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let db = Data::extract(req);
        let auth = req.headers().get("Authorization").cloned();
        Box::pin(async move {
            if let Some(auth) = auth {
                let db: Data<Database> = db.await?;
                let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ") else { return Err(TimeError::Unauthorized) };
                let token = token.to_owned();
                let user = block(move || db.to_owned().get_user_by_token(&token).unwrap()).await;
                if let Ok(user) = user {
                    Ok(UserId { id: user.id })
                } else {
                    Err(TimeError::Unauthorized)
                }
            } else {
                return Err(TimeError::Unauthorized);
            }
        })
    }
}

impl FromRequest for RegisteredUser {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let db = Data::extract(req);
        let auth = req.headers().get("Authorization").cloned();
        Box::pin(async move {
            if let Some(auth) = auth {
                let db: Data<Database> = db.await?;
                let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ") else { return Err(TimeError::Unauthorized) };
                let token = token.to_owned();
                let user = block(move || db.to_owned().get_user_by_token(&token).unwrap()).await;
                if let Ok(user) = user {
                    Ok(user)
                } else {
                    Err(TimeError::Unauthorized)
                }
            } else {
                return Err(TimeError::Unauthorized);
            }
        })
    }
}

#[post("/auth/login")]
pub async fn login(
    data: Json<RegisterRequest>,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    match db.get_user_by_name(&data.username) {
        Ok(user) => match db.verify_user_password(&data.username, &data.password) {
            Ok(true) => {
                let token = json!({ "token": user.auth_token }).to_string();
                Ok(HttpResponse::Ok().body(token))
            }
            Ok(false) => Ok(HttpResponse::Unauthorized().body("Invalid password or username")),
            Err(e) => Err(e),
        },
        Err(_) => Ok(HttpResponse::Unauthorized().body("No such user")),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<Database>) -> Result<impl Responder, TimeError> {
    match db.regenerate_token(user.id) {
        Ok(token) => {
            let token = json!({ "token": token }).to_string();
            Ok(HttpResponse::Ok().body(token))
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/auth/register")]
pub async fn register(data: Json<RegisterRequest>, db: Data<Database>) -> Result<impl Responder> {
    if data.password.len() < 8 || data.password.len() > 128 {
        return Err(actix_web::error::ErrorBadRequest(
            "Password has to be between 8 and 128 characters long",
        ));
    }
    if data.username.len() < 2 || data.username.len() > 32 {
        return Err(actix_web::error::ErrorBadRequest(
            "Username has to be between 2 and 32 characters long",
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
    if data.new.len() < 8 || data.new.len() > 128 {
        return Err(actix_web::error::ErrorBadRequest(
            "Password has to be between 8 and 128 characters long",
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
