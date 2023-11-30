use diesel::prelude::*;

use crate::{error::TimeError, models::*};

impl super::DatabaseWrapper {
    pub async fn search_public_users(&self, search: String) -> Result<Vec<PublicUser>, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;
            Ok(user_identities
                .filter(is_public.eq(true))
                .filter(username.like(format!("%{search}%")))
                .load::<UserIdentity>(&mut conn)?
                .into_iter()
                .map(|u| u.into())
                .collect())
        })
        .await
    }

    pub async fn get_total_user_count(&self) -> Result<u64, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;

            Ok(user_identities.count().first::<i64>(&mut conn)? as u64)
        })
        .await
    }

    pub async fn get_total_coding_time(&self) -> Result<u64, TimeError> {
        self.run_async_query(move |mut conn| {
            use diesel::dsl::sum;

            use crate::schema::coding_activities::dsl::*;

            Ok(coding_activities
                .select(sum(duration))
                .first::<Option<i64>>(&mut conn)?
                .unwrap_or_default() as u64)
        })
        .await
    }
}
