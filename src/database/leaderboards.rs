use std::collections::HashMap;

use chrono::prelude::*;
use diesel::{insert_into, prelude::*};

use crate::{api::users::ListLeaderboard, error::TimeError, models::*};

impl super::DatabaseWrapper {
    pub async fn delete_activity(&self, userid: i32, activity: i32) -> Result<bool, TimeError> {
        use crate::schema::coding_activities::dsl::*;

        let res = self
            .run_async_query(move |mut conn| {
                Ok(diesel::delete(crate::schema::coding_activities::table)
                    .filter(id.eq(activity))
                    .filter(user_id.eq(userid))
                    .execute(&mut conn)?)
            })
            .await?;

        Ok(res != 0)
    }

    pub async fn create_leaderboard(
        &self,
        creator_id: i32,
        name: &str,
    ) -> Result<String, TimeError> {
        let code = crate::utils::generate_token();
        let board = NewLeaderboard {
            name: name.to_string(),
            creation_time: chrono::Local::now().naive_local(),
            invite_code: code.clone(),
        };

        let lid = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboards::dsl::*;

                Ok(insert_into(crate::schema::leaderboards::table)
                    .values(&board)
                    .returning(id)
                    .get_results(&mut conn)?[0])
            })
            .await?;

        let admin = NewLeaderboardMember {
            user_id: creator_id,
            admin: true,
            leaderboard_id: lid,
        };

        self.run_async_query(move |mut conn| {
            insert_into(crate::schema::leaderboard_members::table)
                .values(admin)
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(code)
    }

    pub async fn regenerate_leaderboard_invite(&self, lid: i32) -> Result<String, TimeError> {
        let newinvite = crate::utils::generate_token();

        let newinvite_clone = newinvite.clone();

        self.run_async_query(move |mut conn| {
            use crate::schema::leaderboards::dsl::*;
            diesel::update(crate::schema::leaderboards::table)
                .filter(id.eq(lid))
                .set(invite_code.eq(newinvite_clone))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?;

        Ok(newinvite)
    }

    pub async fn delete_leaderboard(&self, lname: String) -> Result<bool, TimeError> {
        let res = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboards::dsl::*;
                Ok(diesel::delete(crate::schema::leaderboards::table)
                    .filter(name.eq(lname))
                    .execute(&mut conn)?)
            })
            .await?;

        Ok(res != 0)
    }

    pub async fn get_leaderboard_id_by_name(&self, lname: String) -> Result<i32, TimeError> {
        self.run_async_query(move |mut conn| {
            sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
            use crate::schema::leaderboards::dsl::*;

            Ok(leaderboards
                .filter(lower(name).eq(lname.to_lowercase()))
                .select(id)
                .first::<i32>(&mut conn)?)
        })
        .await
    }

    pub async fn get_leaderboard(&self, lname: String) -> Result<PrivateLeaderboard, TimeError> {
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
        let board = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboards::dsl::*;

                Ok(leaderboards
                    .filter(lower(name).eq(lname.to_lowercase()))
                    .first::<Leaderboard>(&mut conn)?)
            })
            .await?;

        let members = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboard_members::dsl::*;

                Ok(leaderboard_members
                    .filter(leaderboard_id.eq(board.id))
                    .load::<LeaderboardMember>(&mut conn)?)
            })
            .await?;

        let mut fullmembers = Vec::new();
        let aweekago = NaiveDateTime::new(
            chrono::Local::today().naive_local() - chrono::Duration::weeks(1),
            chrono::NaiveTime::from_num_seconds_from_midnight(0, 0),
        );

        for m in members {
            if let Ok(user) = self.get_user_by_id(m.user_id).await {
                fullmembers.push(PrivateLeaderboardMember {
                    id: m.id,
                    username: user.username,
                    admin: m.admin,
                    time_coded: self
                        .get_user_coding_time_since(m.user_id, aweekago)
                        .await
                        .unwrap_or(0),
                });
            }
        }
        Ok(PrivateLeaderboard {
            name: board.name,
            invite: board.invite_code,
            creation_time: board.creation_time,
            members: fullmembers,
        })
    }

    pub async fn add_user_to_leaderboard(
        &self,
        uid: i32,
        invite: String,
    ) -> Result<crate::api::users::MinimalLeaderboard, TimeError> {
        let (lid, name) = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboards::dsl::*;
                Ok(leaderboards
                    .filter(invite_code.eq(invite))
                    .select((id, name))
                    .first::<(i32, String)>(&mut conn)?)
            })
            .await?;

        let user = NewLeaderboardMember {
            user_id: uid,
            leaderboard_id: lid,
            admin: false,
        };

        self.run_async_query(move |mut conn| {
            insert_into(crate::schema::leaderboard_members::table)
                .values(&user)
                .execute(&mut conn)?;
            Ok(())
        })
        .await?;

        let member_count: i32 = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboard_members::dsl::*;
                Ok(leaderboard_members
                    .filter(leaderboard_id.eq(lid))
                    .select(diesel::dsl::count(user_id))
                    .first::<i64>(&mut conn)? as i32)
            })
            .await?;

        Ok(crate::api::users::MinimalLeaderboard { name, member_count })
    }

    pub async fn remove_user_from_leaderboard(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let res = self
            .run_async_query(move |mut conn| {
                Ok(diesel::delete(crate::schema::leaderboard_members::table)
                    .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
                    .execute(&mut conn)?)
            })
            .await?;

        Ok(res != 0)
    }

    pub async fn promote_user_to_leaderboard_admin(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let res = self
            .run_async_query(move |mut conn| {
                Ok(diesel::update(crate::schema::leaderboard_members::table)
                    .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
                    .set(admin.eq(true))
                    .execute(&mut conn)?)
            })
            .await?;

        Ok(res != 0)
    }

    pub async fn demote_user_to_leaderboard_member(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        let res = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboard_members::dsl::*;

                Ok(diesel::update(crate::schema::leaderboard_members::table)
                    .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
                    .set(admin.eq(false))
                    .execute(&mut conn)?)
            })
            .await?;
        Ok(res != 0)
    }

    pub async fn is_leaderboard_member(&self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::leaderboard_members::dsl::*;

            Ok(leaderboard_members
                .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
                .select(id)
                .first::<i32>(&mut conn)
                .optional()?
                .is_some())
        })
        .await
    }

    pub async fn is_leaderboard_admin(&self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        self.run_async_query(move |mut conn| {
            Ok(leaderboard_members
                .filter(leaderboard_id.eq(lid).and(user_id.eq(uid)))
                .select(admin)
                .first::<bool>(&mut conn)
                .optional()?
                .unwrap_or(false))
        })
        .await
    }

    pub async fn get_leaderboard_admin_count(&self, lid: i32) -> Result<i64, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        self.run_async_query(move |mut conn| {
            Ok(leaderboard_members
                .filter(leaderboard_id.eq(lid).and(admin.eq(true)))
                .select(diesel::dsl::count(user_id))
                .first::<i64>(&mut conn)?)
        })
        .await
    }

    pub async fn get_user_leaderboards(
        &self,
        uid: i32,
    ) -> Result<Vec<crate::api::users::ListLeaderboard>, TimeError> {
        let ids = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboard_members::dsl::*;

                Ok(leaderboard_members
                    .filter(user_id.eq(uid))
                    .select(leaderboard_id)
                    .order_by(leaderboard_id.asc())
                    .load::<i32>(&mut conn)?)
            })
            .await?;

        let members = self
            .run_async_query(move |mut conn| {
                use crate::schema::leaderboards::dsl::*;
                let ls = leaderboards
                    .filter(id.eq_any(&ids))
                    .order_by(id.asc())
                    .load::<Leaderboard>(&mut conn)?;

                let mut members: HashMap<Leaderboard, Vec<LeaderboardMember>> = HashMap::new();
                for l in &ls {
                    members.insert(l.clone(), {
                        use crate::schema::leaderboard_members::dsl::*;
                        leaderboard_members
                            .filter(leaderboard_id.eq(l.id))
                            .load::<LeaderboardMember>(&mut conn)?
                    });
                }

                Ok(members)
            })
            .await?;

        let aweekago = NaiveDateTime::new(
            chrono::Local::today().naive_local() - chrono::Duration::weeks(1),
            chrono::NaiveTime::from_num_seconds_from_midnight(0, 0),
        );

        // FIXME: cache members
        let mut fullmembers = HashMap::new();
        for (l, ms) in members {
            let mut full = Vec::new();
            for m in ms {
                if let Ok(user) = self.get_user_by_id(m.user_id).await {
                    full.push(PrivateLeaderboardMember {
                        id: m.id,
                        username: user.username,
                        admin: m.admin,
                        time_coded: self
                            .get_user_coding_time_since(m.user_id, aweekago)
                            .await
                            .unwrap_or(0),
                    });
                }
            }
            fullmembers.insert(l, full);
        }

        let mut ret = Vec::new();
        for (l, mut ms) in fullmembers {
            ms.sort_by_key(|m| m.time_coded);
            // NOTE: Leaderboards can't be empty here as they have to contain user
            let mypos = ms.iter().position(|m| m.id == uid).unwrap();
            let me = ms.get(mypos).unwrap();
            let top = ms.last().unwrap();
            ret.push(ListLeaderboard {
                name: l.name,
                me: me.clone(),
                my_position: (mypos + 1) as i32,
                member_count: ms.len() as i32,
                top_member: top.clone(),
            })
        }

        Ok(ret)
    }
}
