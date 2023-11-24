use std::{future::Future, pin::Pin, sync::Arc};

use actix_web::{
    dev::Payload,
    web::{block, Data},
    FromRequest, HttpRequest,
};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

use crate::error::TimeError;

pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
pub mod misc;

type DatabaseConnection = diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub struct Database {
    backend: Pool<ConnectionManager<PgConnection>>,
}

pub struct DatabaseWrapper {
    db: Arc<Database>,
}

impl FromRequest for DatabaseWrapper {
    type Error = TimeError;
    type Future = Pin<Box<dyn Future<Output = actix_web::Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let wrapper = DatabaseWrapper {
            db: req
                .app_data::<Data<Database>>()
                .unwrap()
                .clone()
                .into_inner(),
        };

        Box::pin(async move { Ok(wrapper) })
    }
}

impl Database {
    fn get(&self) -> Result<DatabaseConnection, TimeError> {
        Ok(self.backend.get()?)
    }

    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { backend: pool }
    }
}

impl DatabaseWrapper {
    async fn run_async_query<
        T: Send + 'static,
        F: FnOnce(DatabaseConnection) -> Result<T, TimeError> + Send + 'static,
    >(
        &self,
        query: F,
    ) -> Result<T, TimeError> {
        let conn = self.db.get()?;

        block(move || query(conn)).await?
    }
}
