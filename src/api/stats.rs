use axum::Json;
use serde_derive::Serialize;
use utoipa::ToSchema;

use crate::{database::DatabaseWrapper, error::TimeError};

/// Global Testaustime statistics.
#[derive(Serialize, ToSchema)]
pub struct Stats {
    /// Total number of registered users.
    pub user_count: u64,
    /// Total coding time tracked in seconds.
    pub coding_time: u64,
}

/// Get global Testaustime statistics.
///
/// Returns the total number of registered users and total coding time tracked.
#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = OK, description = "Global statistics", body = Stats),
    )
)]
pub async fn stats(db: DatabaseWrapper) -> Result<Json<Stats>, TimeError> {
    let user_count = db.get_total_user_count().await?;
    let coding_time = db.get_total_coding_time().await?;

    Ok(Json(Stats {
        user_count,
        coding_time,
    }))
}
