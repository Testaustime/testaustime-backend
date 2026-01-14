use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{serde::ts_seconds_option, DateTime, Duration, Local, Utc};
use http::StatusCode;
use serde::Serialize;
use serde_derive::Deserialize;
use utoipa::ToSchema;

use crate::{
    api::{activity::HeartBeatMemoryStore, auth::UserIdentityOptional},
    database::DatabaseWrapper,
    error::TimeError,
    models::{CodingActivity, CurrentActivity, PrivateLeaderboardMember, UserId, UserIdentity},
    utils::group_by_language,
};

#[derive(Deserialize, Debug)]
pub struct DataRequest {
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub to: Option<DateTime<Utc>>,
    pub min_duration: Option<i32>,
    pub editor_name: Option<String>,
    pub language: Option<String>,
    pub hostname: Option<String>,
    pub project_name: Option<String>,
}

#[utoipa::path(
    get,
    path = "/users/@me",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = UserIdentity)
    )
)]
pub async fn my_profile(user: UserIdentity) -> Result<Json<UserIdentity>, TimeError> {
    Ok(Json(user))
}

#[derive(Serialize, ToSchema)]
pub struct ListLeaderboard {
    pub name: String,
    pub member_count: i32,
    pub top_member: PrivateLeaderboardMember,
    pub my_position: i32,
    pub me: PrivateLeaderboardMember,
}

#[derive(Serialize, ToSchema)]
pub struct MinimalLeaderboard {
    pub name: String,
    pub member_count: i32,
}

#[utoipa::path(
    get,
    path = "/users/@me/leaderboards",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = Vec<ListLeaderboard>)
    )
)]
pub async fn my_leaderboards(
    user: UserId,
    db: DatabaseWrapper,
) -> Result<Json<Vec<ListLeaderboard>>, TimeError> {
    Ok(Json(db.get_user_leaderboards(user.id).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct UserAuthentication {
    pub username: String,
    pub password: String,
}

#[utoipa::path(
    delete,
    path = "/users/@me/delete",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn delete_user(
    db: DatabaseWrapper,
    user: Json<UserAuthentication>,
) -> Result<StatusCode, TimeError> {
    if let Some(user) = db
        .verify_user_password(&user.username, &user.password)
        .await?
    {
        db.delete_user(user.id).await?;
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::Unauthorized)
    }
}

#[utoipa::path(
    get,
    path = "/users/{username}/activity/current",
    params(
        ("username", description = "User name"),
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = Option<CurrentActivity>)
    )
)]
pub async fn get_current_activity(
    Path(name): Path<String>,
    opt_user: UserIdentityOptional,
    db: DatabaseWrapper,
    State(heartbeats): State<Arc<HeartBeatMemoryStore>>,
) -> Result<Json<Option<CurrentActivity>>, TimeError> {
    let mut is_self = false;

    let target_user = if let Some(user) = opt_user.identity {
        if name == "@me" {
            user.id
        } else {
            let target_user = db
                .get_user_by_name(&name)
                .await
                .map_err(|_| TimeError::UserNotFound)?;

            if target_user.id == user.id
                || target_user.is_public
                || db.are_friends(user.id, target_user.id).await?
            {
                is_self = target_user.id == user.id;
                target_user.id
            } else {
                return Err(TimeError::Unauthorized);
            }
        }
    } else {
        let target_user = db
            .get_user_by_name(&name)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if target_user.is_public {
            target_user.id
        } else {
            return Err(TimeError::UserNotFound);
        }
    };

    match heartbeats.get(&target_user) {
        Some(heartbeat) => {
            let (inner_heartbeat, start, duration) = heartbeat.to_owned();
            drop(heartbeat);
            let curtime = Local::now().naive_local();
            if curtime.signed_duration_since(start + duration) > Duration::seconds(900) {
                db.add_activity(target_user, inner_heartbeat, start, duration)
                    .await?;

                heartbeats.remove(&target_user);
                Err(TimeError::NotActive)
            } else {
                let mut current_heartbeat = CurrentActivity {
                    started: start,
                    duration: duration.num_seconds(),
                    heartbeat: inner_heartbeat,
                };

                if !is_self && current_heartbeat.heartbeat.hidden == Some(true) {
                    current_heartbeat.heartbeat.project_name = Some(String::from("hidden"));
                }

                Ok(Json(Some(current_heartbeat)))
            }
        }
        None => Err(TimeError::NotActive),
    }
}

#[utoipa::path(
    get,
    path = "/users/{username}/activity/data",
    params(
        ("username", description = "User name"),
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = Vec<CodingActivity>)
    )
)]
pub async fn get_activities(
    db: DatabaseWrapper,
    Query(data): Query<DataRequest>,
    Path(name): Path<String>,
    opt_user: UserIdentityOptional,
) -> Result<Json<Vec<CodingActivity>>, TimeError> {
    let Some(user) = opt_user.identity else {
        let target_user = db
            .get_user_by_name(&name)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if target_user.is_public {
            return Ok(Json(db.get_activity(data, target_user.id, false).await?));
        } else {
            return Err(TimeError::UserNotFound);
        };
    };

    let data: Vec<CodingActivity> = if name == "@me" {
        db.get_activity(data, user.id, true).await?
    } else {
        //FIXME: This is technically not required when the username equals the username of the
        //authenticated user
        let target_user = db
            .get_user_by_name(&name)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if target_user.id == user.id
            || target_user.is_public
            || db.are_friends(user.id, target_user.id).await?
        {
            db.get_activity(data, target_user.id, target_user.id == user.id)
                .await?
        } else {
            return Err(TimeError::Unauthorized);
        }
    };

    Ok(Json(data))
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LanguageSummary {
    languages: HashMap<String, i32>,
    total: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ActivitySummary {
    last_week: LanguageSummary,
    last_month: LanguageSummary,
    all_time: LanguageSummary,
}

#[utoipa::path(
    get,
    path = "/users/{username}/activity/summary",
    params(
        ("username", description = "User name")
    ),
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = ActivitySummary)
    )
)]
pub async fn get_activity_summary(
    Path(path): Path<String>,
    opt_user: UserIdentityOptional,
    db: DatabaseWrapper,
) -> Result<Json<ActivitySummary>, TimeError> {
    let data = if let Some(user) = opt_user.identity {
        if path == "@me" {
            db.get_all_activity(user.id).await?
        } else {
            let target_user = db
                .get_user_by_name(&path)
                .await
                .map_err(|_| TimeError::UserNotFound)?;

            if target_user.id == user.id
                || target_user.is_public
                || db.are_friends(user.id, target_user.id).await?
            {
                db.get_all_activity(target_user.id).await?
            } else {
                return Err(TimeError::Unauthorized);
            }
        }
    } else {
        let target_user = db
            .get_user_by_name(&path)
            .await
            .map_err(|_| TimeError::UserNotFound)?;

        if target_user.is_public {
            db.get_all_activity(target_user.id).await?
        } else {
            return Err(TimeError::UserNotFound);
        }
    };

    //FIXME: This does a lot of unnecessary calculations
    let now = Local::now().naive_local();

    let all_time = group_by_language(data.clone().into_iter());
    let last_month = group_by_language(
        data.clone()
            .into_iter()
            .filter(|d| now.signed_duration_since(d.start_time) < Duration::days(30)),
    );
    let last_week = group_by_language(
        data.into_iter()
            .filter(|d| now.signed_duration_since(d.start_time) < Duration::days(7)),
    );

    Ok(Json(ActivitySummary {
        last_week: LanguageSummary {
            total: last_week.values().sum::<i32>(),
            languages: last_week,
        },
        last_month: LanguageSummary {
            total: last_month.values().sum::<i32>(),
            languages: last_month,
        },
        all_time: LanguageSummary {
            total: all_time.values().sum::<i32>(),
            languages: all_time,
        },
    }))
}
