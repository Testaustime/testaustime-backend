use axum::Json;
use serde_derive::Serialize;
use utoipa::ToSchema;

use crate::{database::DatabaseWrapper, error::TimeError};

#[derive(Serialize, ToSchema)]
pub struct Stats {
    pub user_count: u64,
    pub coding_time: u64,
}

#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = OK, body = Stats)
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
