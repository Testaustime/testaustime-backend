use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{block, Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder,
};

use crate::{
    database::DatabaseConnection,
    error::TimeError,
    models::{SelfUser, UserId, UserIdentity},
    requests::*,
    DbPool,
};

impl FromRequest for UserId {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<UserId, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let db = Data::<DbPool>::extract(req);
        let auth = req.headers().get("Authorization").cloned();
        Box::pin(async move {
            if let Some(auth) = auth {
                let db: Data<DbPool> = db.await?;
                let user = block(move || {
                    let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ").to_owned() else { return Err(TimeError::Unauthorized) };
                    db.get()?.get_user_by_token(token)
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

impl FromRequest for UserIdentity {
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
                    db.get()?.get_user_by_token(token)
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
    match block(move || {
        db.get()?
            .verify_user_password(&data.username, &data.password)
    })
    .await?
    {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        Ok(None) => Err(TimeError::Unauthorized),
        Err(e) => Err(e),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<DbPool>) -> Result<impl Responder, TimeError> {
    match block(move || db.get()?.regenerate_token(user.id)).await? {
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

    let mut conn = db.get()?;
    let username = data.username.clone();
    if block(move || conn.get_user_by_name(&username))
        .await?
        .is_ok()
    {
        return Err(TimeError::UserExists);
    }

    Ok(Json(
        block(move || {
            db.get()?
                .new_testaustime_user(&data.username, &data.password)
        })
        .await??,
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
    if !super::VALID_NAME_REGEX.is_match(&data.new) {
        return Err(TimeError::BadUsername);
    }
    let mut conn = db.get()?;
    let username = data.new.clone();
    if block(move || conn.get_user_by_name(&username))
        .await?
        .is_ok()
    {
        return Err(TimeError::UserExists);
    }

    let mut conn = db.get()?;
    match block(move || conn.get_user_by_id(userid.id)).await? {
        Ok(user) => match block(move || db.get()?.change_username(user.id, &data.new)).await? {
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
    user: UserIdentity,
    data: Json<PasswordChangeRequest>,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 8 || data.new.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    let old = data.old.to_owned();
    let mut conn = db.get()?;
    let mut conn2 = db.get()?;
    let tuser = block(move || db.get()?.get_testaustime_user_by_id(user.id)).await??;
    let k = block(move || conn.verify_user_password(&user.username, &old)).await??;
    if k.is_some() || tuser.password.iter().all(|n| *n == 0) {
        match conn2.change_password(user.id, &data.new) {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Err(e),
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}
