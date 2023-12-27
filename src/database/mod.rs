use std::{future::Future, pin::Pin, sync::Arc};

use actix_web::{dev::Payload, web::Data, FromRequest, HttpRequest};
use diesel_async::{
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};

use crate::error::TimeError;

pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
pub mod misc;

type DatabaseConnection = Object<AsyncPgConnection>;

pub struct Database {
    backend: Pool<AsyncPgConnection>,
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
    async fn get(&self) -> Result<DatabaseConnection, TimeError> {
        Ok(self.backend.get().await?)
    }

    pub fn new(url: String) -> Self {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);

        let pool = Pool::builder(manager)
            .build()
            .expect("Failed to create connection pool");

        Self { backend: pool }
    }
}
