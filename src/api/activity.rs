use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use chrono::{Duration, Local};
use dashmap::DashMap;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    api::auth::SecuredUserIdentity,
    database::DatabaseWrapper,
    error::TimeError,
    models::{HeartBeat, UserId},
};

pub type HeartBeatMemoryStore = DashMap<i32, (HeartBeat, chrono::NaiveDateTime, chrono::Duration)>;

#[derive(Deserialize)]
pub struct ActivityRenameRequest {
    from: String,
    to: String,
}

#[derive(Deserialize)]
pub struct HideRequest {
    target_project: String,
    hidden: bool,
}

#[derive(Serialize)]
pub struct UpdateResponse {
    duration: i64,
}
pub async fn update(
    user: UserId,
    db: DatabaseWrapper,
    heartbeats: State<Arc<HeartBeatMemoryStore>>,
    Json(heartbeat): Json<HeartBeat>,
) -> Result<impl IntoResponse, TimeError> {
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

pub async fn flush(
    user: UserId,
    db: DatabaseWrapper,
    heartbeats: State<Arc<HeartBeatMemoryStore>>,
) -> Result<impl IntoResponse, TimeError> {
    if let Some(heartbeat) = heartbeats.get(&user.id) {
        let (inner_heartbeat, start, duration) = heartbeat.to_owned();
        drop(heartbeat);
        heartbeats.remove(&user.id);
        db.add_activity(user.id, inner_heartbeat, start, duration)
            .await?;
    }
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ActivityDeleteRequest {
    id: i32,
}

pub async fn delete(
    user: SecuredUserIdentity,
    db: DatabaseWrapper,
    Json(body): Json<ActivityDeleteRequest>,
) -> Result<impl IntoResponse, TimeError> {
    let deleted = db.delete_activity(user.identity.id, body.id).await?;
    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err(TimeError::BadId)
    }
}

pub async fn rename_project(
    user: UserId,
    db: DatabaseWrapper,
    body: Json<ActivityRenameRequest>,
) -> Result<impl IntoResponse, TimeError> {
    let renamed = db.rename_project(user.id, &body.from, &body.to).await?;

    Ok(Json(json!({ "affected_activities": renamed })))
}

pub async fn hide_project(
    user: UserId,
    db: DatabaseWrapper,
    body: Json<HideRequest>,
) -> Result<impl IntoResponse, TimeError> {
    let renamed = db
        .set_project_hidden(user.id, &body.target_project, body.hidden)
        .await?;

    Ok(Json(json!({ "affected_activities": renamed })))
}
