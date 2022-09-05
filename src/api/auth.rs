use std::{future::Future, pin::Pin};

use actix_web::{
    dev::Payload,
    error::*,
    web::{block, Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder,
};
use database::Database;

use crate::{
    database::{
        self, change_password, change_username, get_testaustime_user_by_id, get_user_by_id,
        get_user_by_token, new_testaustime_user, regenerate_token, verify_user_password,
    },
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
                    get_user_by_token(&mut db.get()?, token)
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
    match block(move || verify_user_password(&mut db.get()?, &data.username, &data.password))
        .await?
    {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        Ok(None) => Err(TimeError::Unauthorized),
        Err(e) => Err(e),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: Data<DbPool>) -> Result<impl Responder, TimeError> {
    match block(move || regenerate_token(&mut db.get()?, user.id)).await? {
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
    if block(move || database::get_user_by_name(&mut conn, &username))
        .await?
        .is_ok()
    {
        return Err(TimeError::UserExists);
    }

    Ok(Json(
        block(move || new_testaustime_user(&mut db.get()?, &data.username, &data.password))
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
    if block(move || database::get_user_by_name(&mut conn, &username))
        .await?
        .is_ok()
    {
        return Err(TimeError::UserExists);
    }

    let mut conn = db.get()?;
    match block(move || get_user_by_id(&mut conn, userid.id)).await? {
        Ok(user) => match block(move || change_username(&mut db.get()?, user.id, &data.new)).await?
        {
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
    let tuser = block(move || get_testaustime_user_by_id(&mut db.get()?, user.id)).await??;
    let k = block(move || verify_user_password(&mut conn, &user.username, &old)).await??;
    if k.is_some() || tuser.password.iter().all(|n| *n == 0) {
        match change_password(&mut conn2, user.id, &data.new) {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Err(e),
        }
    } else {
        Err(TimeError::Unauthorized)
    }
}
