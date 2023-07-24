use std::{future::Future, pin::Pin};

use actix_web::{
    dev::{ConnectionInfo, Payload},
    error::*,
    web::{Data, Json},
    FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder,
};

use crate::{
    auth::Authentication,
    database::DatabaseWrapper,
    error::TimeError,
    models::{SelfUser, UserId, UserIdentity},
    requests::*,
    RegisterLimiter,
};

impl FromRequest for UserId {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let auth = req.extensions().get::<Authentication>().cloned().unwrap();
        Box::pin(async move {
            if let Authentication::AuthToken(user) = auth {
                Ok(UserId { id: user.id })
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
        let auth = req.extensions().get::<Authentication>().cloned().unwrap();
        Box::pin(async move {
            if let Authentication::AuthToken(user) = auth {
                Ok(user)
            } else {
                Err(TimeError::Unauthorized)
            }
        })
    }
}

pub struct UserIdentityOptional {
    pub identity: Option<UserIdentity>,
}

impl FromRequest for UserIdentityOptional {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let auth = req.extensions().get::<Authentication>().cloned().unwrap();
        Box::pin(async move {
            if let Authentication::AuthToken(user) = auth {
                Ok(UserIdentityOptional {
                    identity: Some(user),
                })
            } else {
                Ok(UserIdentityOptional { identity: None })
            }
        })
    }
}

#[post("/auth/login")]
pub async fn login(
    data: Json<RegisterRequest>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    if data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password cannot be longer than 128 characters".to_string(),
        ));
    }
    match db
        .verify_user_password(&data.username, &data.password)
        .await
    {
        Ok(Some(user)) => Ok(Json(SelfUser::from(user))),
        _ => Err(TimeError::InvalidCredentials),
    }
}

#[post("/auth/regenerate")]
pub async fn regenerate(user: UserId, db: DatabaseWrapper) -> Result<impl Responder, TimeError> {
    db.regenerate_token(user.id)
        .await
        .inspect_err(|e| error!("{}", e))
        .map(|token| {
            let token = json!({ "token": token });
            Json(token)
        })
}

#[post("/auth/register")]
pub async fn register(
    conn_info: ConnectionInfo,
    data: Json<RegisterRequest>,
    db: DatabaseWrapper,
    rls: Data<RegisterLimiter>,
) -> Result<impl Responder, TimeError> {
    if data.password.len() < 8 || data.password.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    if !super::VALID_NAME_REGEX.is_match(&data.username) {
        return Err(TimeError::BadUsername);
    }

    let username = data.username.clone();
    if db.get_user_by_name(username).await.is_ok() {
        return Err(TimeError::UserExists);
    }

    let ip = if rls.limit_by_peer_ip {
        conn_info.peer_addr().ok_or(TimeError::UnknownError)?
    } else {
        conn_info
            .realip_remote_addr()
            .ok_or(TimeError::UnknownError)?
    };

    if let Some(res) = rls.storage.get(ip) {
        if chrono::Local::now()
            .naive_local()
            .signed_duration_since(*res)
            < chrono::Duration::days(1)
        {
            return Err(TimeError::TooManyRegisters);
        }
    }

    let res = db
        .new_testaustime_user(&data.username, &data.password)
        .await?;

    rls.storage
        .insert(ip.to_string(), chrono::Local::now().naive_local());

    Ok(Json(res))
}

#[post("/auth/changeusername")]
pub async fn changeusername(
    userid: UserId,
    data: Json<UsernameChangeRequest>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 2 || data.new.len() > 32 {
        return Err(TimeError::InvalidLength(
            "Username is not between 2 and 32 chars".to_string(),
        ));
    }
    if !super::VALID_NAME_REGEX.is_match(&data.new) {
        return Err(TimeError::BadUsername);
    }

    let username = data.new.clone();
    if db.get_user_by_name(username).await.is_ok() {
        return Err(TimeError::UserExists);
    }

    let user = db.get_user_by_id(userid.id).await?;
    db.change_username(user.id, data.new.clone()).await?;
    Ok(HttpResponse::Ok().finish())
}

#[post("/auth/changepassword")]
pub async fn changepassword(
    user: UserIdentity,
    data: Json<PasswordChangeRequest>,
    db: DatabaseWrapper,
) -> Result<impl Responder, TimeError> {
    if data.new.len() < 8 || data.new.len() > 128 {
        return Err(TimeError::InvalidLength(
            "Password has to be between 8 and 128 characters long".to_string(),
        ));
    }
    let old = data.old.to_owned();
    let tuser = db.get_testaustime_user_by_id(user.id).await?;
    let k = db.verify_user_password(&user.username, &old).await?;
    if k.is_some() || tuser.password.iter().all(|n| *n == 0) {
        db.change_password(user.id, &data.new)
            .await
            .map(|_| HttpResponse::Ok().finish())
    } else {
        Err(TimeError::Unauthorized)
    }
}
