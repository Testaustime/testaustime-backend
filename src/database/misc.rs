use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{error::TimeError, models::*};

impl super::DatabaseWrapper {
    pub async fn search_public_users(&self, search: String) -> Result<Vec<PublicUser>, TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::user_identities::dsl::*;
        Ok(user_identities
            .filter(is_public.eq(true))
            .filter(username.like(format!("%{search}%")))
            .load::<UserIdentity>(&mut conn)
            .await?
            .into_iter()
            .map(|u| u.into())
            .collect())
    }

    pub async fn get_total_user_count(&self) -> Result<u64, TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::user_identities::dsl::*;
        Ok(user_identities.count().first::<i64>(&mut conn).await? as u64)
    }

    pub async fn get_total_coding_time(&self) -> Result<u64, TimeError> {
        let mut conn = self.db.get().await?;

        use diesel::dsl::sum;

        use crate::schema::coding_activities::dsl::*;

        Ok(coding_activities
            .select(sum(duration))
            .first::<Option<i64>>(&mut conn)
            .await?
            .unwrap_or_default() as u64)
    }
}
