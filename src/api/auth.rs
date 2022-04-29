use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{block, Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder,
};

use crate::{
    database::{
        change_password, change_username, get_user_by_id, get_user_by_token, new_user,
        regenerate_token, verify_user_password,
    },
    error::TimeError,
    models::{RegisteredUser, SelfUser, UserId},
    requests::*,
    DbPool,
};

impl FromRequest for UserId {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<UserId, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let db = Data::extract(req);
        let auth = req.headers().get("Authorization").cloned();
        Box::pin(async move {
            if let Some(auth) = auth {
                let db: Data<DbPool> = db.await?;
                let user = block(move || {
                    let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ").to_owned() else { return Err(TimeError::Unauthorized) };
                    get_user_by_token(&db.get()?, token)
                }).await?;
                if let Ok(user) = user {
                    Ok(UserId { id: user.id })
                } else {
                    Err(TimeError::Unauthorized)
                }
            } else {
                Err(TimeError::Unauthorized)
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
                let db: Data<DbPool> = db.await?;
                let user = block(move || {
                    let Some(token) = auth.to_str().unwrap().strip_prefix("Bearer ") else { return Err(TimeError::Unauthorized) };
                    get_user_by_token(&db.get()?, token)
                }).await?;
                if let Ok(user) = user {
                    Ok(user)
                } else {
                    Err(TimeError::Unauthorized)
                }
            } else {
                Err(TimeError::Unauthorized)
            }
        })
    }
}

#[post("/auth/login")]
pub async fn login(
    data: Json<RegisterRequest>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password cannot be longer than 128 characters".to_string(),
        ));
    }
    match block(move || verify_user_password(&db.get()?, &data.username, &data.password)).await? {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        Ok(None) => Err(TimeError::Unauthorized),
        Err(e) => Err(e),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<DbPool>) -> Result<impl Responder, TimeError> {
    match block(move || regenerate_token(&db.get()?, user.id)).await? {
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
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if data.password.len() < 8 || data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    if !super::VALID_NAME_REGEX.is_match(&data.username) {
        return Err(TimeError::BadUsername);
    }
    Ok(Json(
        block(move || new_user(&db.get()?, &data.username, &data.password)).await??,
    ))
}

#[post("/auth/changeusername")]
pub async fn changeusername(
    userid: UserId,
    data: Json<UsernameChangeRequest>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 2 || data.new.len() > 32 {
        return Err(TimeError::InvalidLength(
            "Username is not between 2 and 32 chars".to_string(),
        ));
    }

    let conn = db.get()?;
    match block(move || get_user_by_id(&conn, userid.id)).await? {
        Ok(user) => match block(move || change_username(&db.get()?, user.id, &data.new)).await? {
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
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 8 || data.new.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    // FIXME: This whole thing is just horrible
    let old = data.old.to_owned();
    let clone = db.clone();
    let clone2 = db.clone();
    match block(move || get_user_by_id(&clone.get()?, userid.id)).await? {
        Ok(user) => {
            match block(move || verify_user_password(&clone2.get()?, &user.username, &old)).await? {
                Ok(k) => {
                    if k.is_some() || user.password.iter().all(|n| *n == 0) {
                        // Some noobs don't have password (me)
                        match change_password(&db.get()?, userid.id, &data.new) {
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
