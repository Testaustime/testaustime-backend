use std::sync::Arc;

use axum::{Json, extract::State};
use chrono::{Duration, Local};
use dashmap::DashMap;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    database::DatabaseWrapper,
    error::TimeError,
    models::{HeartBeat, UserId, UserIdentity},
};

pub type HeartBeatMemoryStore = DashMap<i32, (HeartBeat, chrono::NaiveDateTime, chrono::Duration)>;

#[derive(Serialize, ToSchema)]
pub struct UpdateResponse {
    duration: i64,
}

#[utoipa::path(
    post,
    path = "/activity/update",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = UpdateResponse)
    )
)]
pub async fn update(
    user: UserId,
    db: DatabaseWrapper,
    heartbeats: State<Arc<HeartBeatMemoryStore>>,
    Json(heartbeat): Json<HeartBeat>,
) -> Result<Json<UpdateResponse>, TimeError> {
    if let Some(project) = &heartbeat.project_name {
        if project.len() > 64 {
            return Err(TimeError::InvalidLength(
                "Project name is over 64 chars".to_string(),
            ));
        }
    }
    if let Some(language) = &heartbeat.language {
        if language.len() > 32 {
            return Err(TimeError::InvalidLength(
                "Language is over 32 chars".to_string(),
            ));
        }
    }
    if let Some(editor) = &heartbeat.editor_name {
        if editor.len() > 32 {
            return Err(TimeError::InvalidLength(
                "Editor name is over 32 chars".to_string(),
            ));
        }
    }
    if let Some(hostname) = &heartbeat.hostname {
        if hostname.len() > 32 {
            return Err(TimeError::InvalidLength(
                "Hostname is over 32 chars".to_string(),
            ));
        }
    }

    match heartbeats.get(&user.id) {
        Some(activity) => {
            let (current_heartbeat, start, mut duration) = activity.to_owned();
            drop(activity);
            let curtime = Local::now().naive_local();
            if heartbeat.eq(&current_heartbeat) {
                if curtime.signed_duration_since(start + duration) > Duration::seconds(900) {
                    // If the user sends a heartbeat but maximum activity duration has been exceeded,
                    // end session and start new
                    db.add_activity(user.id, current_heartbeat, start, duration)
                        .await?;

                    heartbeats.insert(
                        user.id,
                        (heartbeat, Local::now().naive_local(), Duration::seconds(0)),
                    );
                    Ok(Json(UpdateResponse { duration: 0 }))
                } else {
                    // Extend current coding session if heartbeat matches and it has been under the maximum duration of a break
                    heartbeats.insert(
                        user.id,
                        (heartbeat, start, curtime.signed_duration_since(start)),
                    );
                    Ok(Json(UpdateResponse {
                        duration: curtime.signed_duration_since(start).num_seconds(),
                    }))
                }
            } else {
                // Flush current session and start new session if heartbeat changes
                if curtime.signed_duration_since(start + duration) < Duration::seconds(30) {
                    duration = curtime.signed_duration_since(start);
                }

                db.add_activity(user.id, current_heartbeat, start, duration)
                    .await?;

                heartbeats.insert(
                    user.id,
                    (heartbeat, Local::now().naive_local(), Duration::seconds(0)),
                );

                Ok(Json(UpdateResponse { duration: 0 }))
            }
        }
        None => {
            // If the user has not sent a heartbeat during this session
            heartbeats.insert(
                user.id,
                (heartbeat, Local::now().naive_local(), Duration::seconds(0)),
            );
            Ok(Json(UpdateResponse { duration: 0 }))
        }
    }
}

#[utoipa::path(
    post,
    path = "/activity/flush",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn flush(
    user: UserId,
    db: DatabaseWrapper,
    heartbeats: State<Arc<HeartBeatMemoryStore>>,
) -> Result<StatusCode, TimeError> {
    if let Some(heartbeat) = heartbeats.get(&user.id) {
        let (inner_heartbeat, start, duration) = heartbeat.to_owned();
        drop(heartbeat);
        heartbeats.remove(&user.id);
        db.add_activity(user.id, inner_heartbeat, start, duration)
            .await?;
    }
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
pub struct ActivityDeleteRequest {
    id: i32,
}

#[utoipa::path(
    delete,
    path = "/activity/delete",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK)
    )
)]
pub async fn delete(
    user: UserIdentity,
    db: DatabaseWrapper,
    Json(body): Json<ActivityDeleteRequest>,
) -> Result<StatusCode, TimeError> {
    let deleted = db.delete_activity(user.id, body.id).await?;
    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::BadId)
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ActivityRenameRequest {
    from: String,
    to: String,
}

#[derive(Serialize, ToSchema)]
pub struct ActivityRenameResponse {
    affected_activities: usize,
}

#[utoipa::path(
    post,
    path = "/activity/rename",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = ActivityRenameResponse)
    )
)]
pub async fn rename_project(
    user: UserId,
    db: DatabaseWrapper,
    body: Json<ActivityRenameRequest>,
) -> Result<Json<ActivityRenameResponse>, TimeError> {
    let renamed = db.rename_project(user.id, &body.from, &body.to).await?;

    Ok(Json(ActivityRenameResponse {
        affected_activities: renamed,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct HideRequest {
    target_project: String,
    hidden: bool,
}

#[derive(Serialize, ToSchema)]
pub struct HideResponse {
    affected_activities: usize,
}

#[utoipa::path(
    post,
    path = "/activity/hide",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, body = HideResponse)
    )
)]
pub async fn hide_project(
    user: UserId,
    db: DatabaseWrapper,
    body: Json<HideRequest>,
) -> Result<Json<HideResponse>, TimeError> {
    let renamed = db
        .set_project_hidden(user.id, &body.target_project, body.hidden)
        .await?;

    Ok(Json(HideResponse {
        affected_activities: renamed,
    }))
}
