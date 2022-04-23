use actix_web::{
    error::*,
    web::{self, block, Data},
    HttpResponse, Responder,
};
use diesel::result::DatabaseErrorKind;

use crate::{
    database::{self, get_coding_time_steps, get_user_by_name, remove_friend},
    error::TimeError,
    models::{CodingTimeSteps, FriendWithTime, UserId},
    DbPool,
};

#[post("/friends/add")]
pub async fn add_friend(
    user: UserId,
    body: String,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    match block(move || {
        database::add_friend(&conn, user.id, &body.trim().trim_start_matches("ttfc_"))
    })
    .await?
    {
        // This is not correct
        Err(e) => {
            error!("{}", e);
            Err(match e {
                TimeError::DieselError(diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    ..,
                )) => TimeError::AlreadyFriends,
                _ => e,
            })
        }
        Ok(friend) => {
            let friend_with_time = FriendWithTime {
                username: friend.username.clone(),
                coding_time: match &db.get() {
                    Ok(c) => get_coding_time_steps(c, friend.id),
                    _ => CodingTimeSteps {
                        all_time: 0,
                        past_month: 0,
                        past_week: 0,
                    },
                },
            };

            Ok(web::Json(friend_with_time))
        }
    }
}

#[get("/friends/list")]
pub async fn get_friends(user: UserId, db: Data<DbPool>) -> Result<impl Responder, TimeError> {
    let conn = db.get()?;
    match block(move || database::get_friends(&conn, user.id)).await? {
        Ok(friends) => {
            let friends_with_time: Vec<FriendWithTime> = friends
                .iter()
                .map(|friend| FriendWithTime {
                    username: friend.username.clone(),
                    coding_time: match &db.get() {
                        Ok(c) => get_coding_time_steps(c, friend.id),
                        _ => CodingTimeSteps {
                            all_time: 0,
                            past_month: 0,
                            past_week: 0,
                        },
                    },
                })
                .collect();

            Ok(web::Json(friends_with_time))
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[post("/friends/regenerate")]
pub async fn regenerate_friend_code(
    user: UserId,
    db: Data<DbPool>,
) -> Result<impl Responder, TimeError> {
    match block(move || database::regenerate_friend_code(&db.get()?, user.id)).await? {
        Ok(code) => Ok(web::Json(json!({ "friend_code": code }))),
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[delete("/friends/remove")]
pub async fn remove(
    user: UserId,
    db: Data<DbPool>,
    body: String,
) -> Result<impl Responder, TimeError> {
    let clone = db.clone();
    let friend = block(move || get_user_by_name(&clone.get()?, &body)).await??;
    let deleted = block(move || remove_friend(&db.get()?, user.id, friend.id)).await??;
    if deleted {
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(TimeError::BadId)
    }
}
