use diesel::{insert_into, prelude::*};
use futures_util::{
    future::OptionFuture,
    stream::{self, StreamExt},
};

use crate::{error::TimeError, models::*};

impl super::DatabaseWrapper {
    pub async fn add_friend(&self, user: i32, friend: String) -> Result<UserIdentity, TimeError> {
        let Some(friend) = self
            .run_async_query(move |mut conn| {
                use crate::schema::user_identities::dsl::*;

                Ok(user_identities
                    .filter(friend_code.eq(friend))
                    .first::<UserIdentity>(&mut conn)
                    .optional()?)
            })
            .await?
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

        self.run_async_query(move |mut conn| {
            insert_into(crate::schema::friend_relations::table)
                .values(crate::models::NewFriendRelation {
                    lesser_id: lesser,
                    greater_id: greater,
                })
                .execute(&mut conn)?;
            Ok(())
        })
        .await?;

        Ok(friend)
    }

    #[allow(dead_code)]
    pub async fn get_friends(&self, user: i32) -> Result<Vec<UserIdentity>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };

        let friends = self
            .run_async_query(move |mut conn| {
                Ok(friend_relations
                    .filter(greater_id.eq(user).or(lesser_id.eq(user)))
                    .load::<FriendRelation>(&mut conn)?
                    .iter()
                    .map(
                        |&FriendRelation {
                             lesser_id: other_lesser_id,
                             greater_id: other_greater_id,
                             ..
                         }| {
                            if other_lesser_id == user {
                                other_greater_id
                            } else {
                                other_lesser_id
                            }
                        },
                    )
                    .filter_map(|cur_friend| {
                        user_identities
                            .find(cur_friend)
                            .first::<UserIdentity>(&mut conn)
                            .ok()
                    })
                    .collect())
            })
            .await?;

        Ok(friends)
    }

    pub async fn get_friends_with_time(&self, user: i32) -> Result<Vec<FriendWithTime>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };

        let friends = stream::iter(
            self.run_async_query(move |mut conn| {
                Ok(friend_relations
                    .filter(greater_id.eq(user).or(lesser_id.eq(user)))
                    .load::<FriendRelation>(&mut conn)?
                    .iter()
                    .map(|fr| {
                        if fr.lesser_id == user {
                            fr.greater_id
                        } else {
                            fr.lesser_id
                        }
                    })
                    .collect::<Vec<_>>())
            })
            .await?,
        )
        .then(|cur_friend| async move {
            let opt_friends = self
                .run_async_query(move |mut conn| {
                    Ok(user_identities
                        .find(cur_friend)
                        .first::<UserIdentity>(&mut conn)
                        .ok())
                })
                .await
                .unwrap();

            let future: OptionFuture<_> = opt_friends
                .map(|friend| async {
                    FriendWithTime {
                        coding_time: self.get_coding_time_steps(friend.id).await,
                        user: friend,
                    }
                })
                .into();

            future.await.unwrap()
        })
        .collect::<Vec<FriendWithTime>>()
        .await;

        Ok(friends)
    }

    pub async fn are_friends(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };

        self.run_async_query(move |mut conn| {
            Ok(friend_relations
                .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
                .first::<FriendRelation>(&mut conn)
                .optional()?
                .is_some())
        })
        .await
    }

    pub async fn remove_friend(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };

        self.run_async_query(move |mut conn| {
            Ok(diesel::delete(friend_relations)
                .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
                .execute(&mut conn)?
                != 0)
        })
        .await
    }

    pub async fn regenerate_friend_code(&self, userid: i32) -> Result<String, TimeError> {
        use crate::schema::user_identities::dsl::*;
        let code = crate::utils::generate_friend_code();
        let code_clone = code.clone();

        self.run_async_query(move |mut conn| {
            diesel::update(user_identities.find(userid))
                .set(friend_code.eq(code_clone))
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(code)
    }
}
