use actix_web::{web, Responder};
use serde_derive::Serialize;

use crate::{database::DatabaseWrapper, error::TimeError};

#[derive(Serialize)]
struct Stats {
    pub user_count: u64,
    pub coding_time: u64,
}

#[get("/stats")]
async fn stats(db: DatabaseWrapper) -> Result<impl Responder, TimeError> {
    let user_count = db.get_total_user_count().await?;
    let coding_time = db.get_total_coding_time().await?;

    Ok(web::Json(Stats {
        user_count,
        coding_time,
    }))
}
