use actix_web::{
    web::{self, block, Data},
    Responder,
};
use serde_derive::Serialize;

use crate::{database::Database, error::TimeError};

#[derive(Serialize)]
struct Stats {
    pub user_count: u64,
    pub coding_time: u64,
}

#[get("/stats")]
async fn stats(db: Data<Database>) -> Result<impl Responder, TimeError> {
    let db2 = db.clone();

    let user_count = block(move || db.get()?.get_total_user_count()).await??;
    let coding_time = block(move || db2.get()?.get_total_coding_time()).await??;

    Ok(web::Json(Stats {
        user_count,
        coding_time,
    }))
}
