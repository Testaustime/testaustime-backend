use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder,
};

use crate::{
    database::Database,
    error::TimeError,
    models::{RegisteredUser, SelfUser},
    requests::*,
    user::UserId,
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
                let user = db.to_owned().get_user_by_token(&token);
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
                let user = db.to_owned().get_user_by_token(&token);
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
    match db.verify_user_password(&data.username, &data.password) {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        Ok(None) => Err(TimeError::Unauthorized),
        Err(e) => Err(e),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<Database>) -> Result<impl Responder, TimeError> {
    match db.regenerate_token(user.id) {
        Ok(token) => {
            let token = json!({ "token": token });
            Ok(Json(token))
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/auth/register")]
pub async fn register(
    data: Json<RegisterRequest>,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    if data.password.len() < 8 || data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    if data.username.len() < 2 || data.username.len() > 32 {
        return Err(TimeError::InvalidLength(
            "Username has to be between 8 and 128 characters long".to_string(),
        ));
    }
    Ok(Json(db.new_user(&data.username, &data.password)?))
}

#[post("/auth/changeusername")]
pub async fn changeusername(
    userid: UserId,
    data: Json<UsernameChangeRequest>,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 2 || data.new.len() > 32 {
        return Err(TimeError::InvalidLength(
            "Username is not between 2 and 32 chars".to_string(),
        ));
    }

    match db.get_user_by_id(userid.id) {
        Ok(user) => match db.change_username(user.id, &data.new) {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Err(e),
        },
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/auth/changepassword")]
pub async fn changepassword(
    userid: UserId,
    data: Json<PasswordChangeRequest>,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 8 || data.new.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    // FIXME: This whole thing is just horrible
    match db.get_user_by_id(userid.id) {
        Ok(user) => {
            match db.verify_user_password(&user.username, &data.old) {
                Ok(k) => {
                    if k.is_some() || user.password.iter().all(|n| *n == 0) {
                        // Some noobs don't have password (me)
                        match db.change_password(userid.id, &data.new) {
                            Ok(_) => Ok(HttpResponse::Ok().finish()),
                            Err(e) => Err(e),
                        }
                    } else {
                        Err(TimeError::Unauthorized)
                    }
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}
