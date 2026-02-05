use std::sync::Arc;

use axum::{extract::State, Json};
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

/// Response from heartbeat update.
#[derive(Serialize, ToSchema)]
pub struct UpdateResponse {
    /// Duration of the current coding session in seconds.
    duration: i64,
}

/// Send a heartbeat to track coding activity.
///
/// Editor extensions send heartbeats periodically to track coding sessions.
/// Returns the duration of the current session.
#[utoipa::path(
    post,
    path = "/activity/update",
    request_body = HeartBeat,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Heartbeat recorded", body = UpdateResponse),
        (status = 400, description = "Field exceeds maximum length"),
        (status = 401, description = "Unauthorized"),
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
        if language.len() > 64 {
            return Err(TimeError::InvalidLength(
                "Language is over 64 chars".to_string(),
            ));
        }
    }
    if let Some(editor) = &heartbeat.editor_name {
        if editor.len() > 64 {
            return Err(TimeError::InvalidLength(
                "Editor name is over 64 chars".to_string(),
            ));
        }
    }
    if let Some(hostname) = &heartbeat.hostname {
        if hostname.len() > 64 {
            return Err(TimeError::InvalidLength(
                "Hostname is over 64 chars".to_string(),
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
                    if duration > Duration::zero() {
                        db.add_activity(user.id, current_heartbeat, start, duration)
                            .await?;
                    }

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

                if duration > Duration::zero() {
                    db.add_activity(user.id, current_heartbeat, start, duration)
                        .await?;
                }

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

/// Flush current activity session to database.
///
/// Forces the current in-memory activity session to be saved to the database.
#[utoipa::path(
    post,
    path = "/activity/flush",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Activity flushed to database"),
        (status = 401, description = "Unauthorized"),
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
        if duration > Duration::zero() {
            db.add_activity(user.id, inner_heartbeat, start, duration)
                .await?;
        }
    }
    Ok(StatusCode::OK)
}

/// Request body for deleting a coding activity.
#[derive(Deserialize, ToSchema)]
pub struct ActivityDeleteRequest {
    /// The ID of the activity to delete.
    id: i32,
}

/// Delete a specific coding activity.
///
/// Permanently removes an activity entry by its ID.
#[utoipa::path(
    delete,
    path = "/activity/delete",
    request_body = ActivityDeleteRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Activity deleted"),
        (status = 400, description = "Invalid activity ID"),
        (status = 401, description = "Unauthorized"),
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

/// Request body for renaming a project.
#[derive(Deserialize, ToSchema)]
pub struct ActivityRenameRequest {
    /// The current project name to rename.
    from: String,
    /// The new project name.
    to: String,
}

/// Response from project rename operation.
#[derive(Serialize, ToSchema)]
pub struct ActivityRenameResponse {
    /// Number of activities that were updated.
    affected_activities: usize,
}

/// Rename a project across all activities.
///
/// Updates all activities with the source project name to use the new name.
#[utoipa::path(
    post,
    path = "/activity/rename",
    request_body = ActivityRenameRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Project renamed", body = ActivityRenameResponse),
        (status = 401, description = "Unauthorized"),
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

/// Request body for setting project visibility.
#[derive(Deserialize, ToSchema)]
pub struct HideRequest {
    /// The project name to modify.
    target_project: String,
    /// Whether the project should be hidden from friends and public view.
    hidden: bool,
}

/// Response from project visibility change.
#[derive(Serialize, ToSchema)]
pub struct HideResponse {
    /// Number of activities that were updated.
    affected_activities: usize,
}

/// Set project visibility in activities.
///
/// Marks all activities for a project as hidden or visible to friends and public profiles.
#[utoipa::path(
    post,
    path = "/activity/hide",
    request_body = HideRequest,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = OK, description = "Project visibility updated", body = HideResponse),
        (status = 401, description = "Unauthorized"),
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
