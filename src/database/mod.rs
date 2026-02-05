use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use diesel_async::{
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};
use http::request::Parts;

use crate::{error::TimeError, TestaustimeState};

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

impl From<&Arc<Database>> for DatabaseWrapper {
    fn from(value: &Arc<Database>) -> Self {
        Self {
            db: Arc::clone(value),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for DatabaseWrapper
where
    TestaustimeState: FromRef<S>,
{
    type Rejection = TimeError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = TestaustimeState::from_ref(state);
        let wrapper = DatabaseWrapper {
            db: Arc::clone(&state.database),
        };

        Ok(wrapper)
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
