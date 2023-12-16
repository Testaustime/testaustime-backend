use diesel::{insert_into, prelude::*};
use diesel_async::RunQueryDsl;

use crate::{error::TimeError, models::*};

impl super::DatabaseWrapper {
    pub async fn add_friend(&self, user: i32, friend: String) -> Result<UserIdentity, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;

        let Some(friend) = user_identities
            .filter(friend_code.eq(friend))
            .first::<UserIdentity>(&mut conn)
            .await
            .optional()?
        else {
            return Err(TimeError::UserNotFound);
        };

        if friend.id == user {
            return Err(TimeError::CurrentUser);
        }

        let (lesser, greater) = if user < friend.id {
            (user, friend.id)
        } else {
            (friend.id, user)
        };

        insert_into(crate::schema::friend_relations::table)
            .values(crate::models::NewFriendRelation {
                lesser_id: lesser,
                greater_id: greater,
            })
            .execute(&mut conn)
            .await?;

        Ok(friend)
    }

    #[allow(dead_code)]
    pub async fn get_friends(&self, user: i32) -> Result<Vec<UserIdentity>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };

        let mut conn = self.db.get().await?;

        let friends = friend_relations
            .inner_join(user_identities.on(id.eq(lesser_id).or(id.eq(greater_id))))
            .select(user_identities::all_columns())
            .distinct()
            .filter(id.ne(user))
            .load::<UserIdentity>(&mut conn)
            .await?;

        Ok(friends)
    }

    pub async fn get_friends_with_time(&self, user: i32) -> Result<Vec<FriendWithTime>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };

        let mut conn = self.db.get().await?;

        let friends = friend_relations
            .filter(lesser_id.eq(user).or(greater_id.eq(user)))
            .inner_join(user_identities.on(id.eq(lesser_id).or(id.eq(greater_id))))
            .distinct()
            .filter(id.ne(user))
            .select(user_identities::all_columns())
            .load::<UserIdentity>(&mut conn)
            .await?;

        // FIXME: A further optimization could be applied
        // by calculating the aggrigates (CodingTimeSteps)
        // in the database. This can be done in a single SQL-query
        // but due to limitations with diesel we would have to do it
        // in a separate function called for example: coding_time_steps_for_users(ids: Vec<i32>)
        let friends_with_time = CodingActivity::belonging_to(&friends)
            .load::<CodingActivity>(&mut conn)
            .await?
            .grouped_by(&friends)
            .iter()
            .zip(friends)
            .map(|(d, u)| FriendWithTime {
                user: u,
                coding_time: CodingTimeSteps {
                    all_time: d.iter().map(|a| a.duration).sum(),
                    past_month: d
                        .iter()
                        .map(|a| {
                            if a.start_time
                                >= chrono::Local::now().naive_local() - chrono::Duration::days(30)
                            {
                                a.duration
                            } else {
                                0
                            }
                        })
                        .sum(),
                    past_week: d
                        .iter()
                        .map(|a| {
                            if a.start_time
                                >= chrono::Local::now().naive_local() - chrono::Duration::days(30)
                            {
                                a.duration
                            } else {
                                0
                            }
                        })
                        .sum(),
                },
            })
            .collect::<Vec<_>>();

        Ok(friends_with_time)
    }

    pub async fn are_friends(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };

        let mut conn = self.db.get().await?;

        Ok(friend_relations
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .first::<FriendRelation>(&mut conn)
            .await
            .optional()?
            .is_some())
    }

    pub async fn remove_friend(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };

        let mut conn = self.db.get().await?;

        Ok(diesel::delete(friend_relations)
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .execute(&mut conn)
            .await?
            != 0)
    }

    pub async fn regenerate_friend_code(&self, userid: i32) -> Result<String, TimeError> {
        use crate::schema::user_identities::dsl::*;
        let code = crate::utils::generate_friend_code();
        let code_clone = code.clone();

        let mut conn = self.db.get().await?;

        diesel::update(user_identities.find(userid))
            .set(friend_code.eq(code_clone))
            .execute(&mut conn)
            .await?;

        Ok(code)
    }
}
