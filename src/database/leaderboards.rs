use std::collections::HashMap;

use chrono::prelude::*;
use diesel::{insert_into, prelude::*};
use diesel_async::RunQueryDsl;
use futures_util::TryStreamExt;

use crate::{
    api::users::ListLeaderboard,
    error::TimeError,
    models::*,
    schema::{
        coding_activities,
        leaderboard_members::{self, user_id},
        user_identities,
    },
};

impl super::DatabaseWrapper {
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

        let mut conn = self.db.get().await?;

        conn.build_transaction()
            .read_write()
            .run(|mut conn| {
                Box::pin(async move {
                    use crate::schema::leaderboards::dsl::*;

                    let lid = insert_into(crate::schema::leaderboards::table)
                        .values(&board)
                        .returning(id)
                        .get_results(&mut conn)
                        .await?[0];

                    let admin = NewLeaderboardMember {
                        user_id: creator_id,
                        admin: true,
                        leaderboard_id: lid,
                    };

                    insert_into(crate::schema::leaderboard_members::table)
                        .values(admin)
                        .execute(&mut conn)
                        .await?;

                    Ok::<(), TimeError>(())
                })
            })
            .await?;

        Ok(code)
    }

    pub async fn regenerate_leaderboard_invite(&self, lid: i32) -> Result<String, TimeError> {
        let newinvite = crate::utils::generate_token();

        let newinvite_clone = newinvite.clone();

        let mut conn = self.db.get().await?;

        use crate::schema::leaderboards::dsl::*;
        diesel::update(leaderboards.find(lid))
            .set(invite_code.eq(newinvite_clone))
            .execute(&mut conn)
            .await?;

        Ok(newinvite)
    }

    pub async fn delete_leaderboard(&self, lname: String) -> Result<bool, TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::leaderboards::dsl::*;
        Ok(diesel::delete(crate::schema::leaderboards::table)
            .filter(name.eq(lname))
            .execute(&mut conn)
            .await?
            != 0)
    }

    pub async fn get_leaderboard_id_by_name(&self, lname: String) -> Result<i32, TimeError> {
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
        use crate::schema::leaderboards::dsl::*;

        let mut conn = self.db.get().await?;

        Ok(leaderboards
            .filter(lower(name).eq(lname.to_lowercase()))
            .select(id)
            .first::<i32>(&mut conn)
            .await?)
    }

    pub async fn get_leaderboard(&self, lname: String) -> Result<PrivateLeaderboard, TimeError> {
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
        let mut conn = self.db.get().await?;

        let board = {
            use crate::schema::leaderboards::dsl::*;

            leaderboards
                .filter(lower(name).eq(lname.to_lowercase()))
                .first::<Leaderboard>(&mut conn)
                .await?
        };

        use crate::schema::{
            coding_activities::dsl::start_time,
            user_identities::dsl::{id, user_identities},
        };

        let members = LeaderboardMember::belonging_to(&board)
            .inner_join(user_identities.on(id.eq(user_id)))
            .load::<(LeaderboardMember, UserIdentity)>(&mut conn)
            .await?;

        let aweekago = NaiveDateTime::new(
            Local::now().date_naive() - chrono::Duration::weeks(1),
            chrono::NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap(),
        );

        let members =
            CodingActivity::belonging_to(&members.iter().map(|(_, u)| u).collect::<Vec<_>>())
                .filter(start_time.ge(aweekago))
                .load::<CodingActivity>(&mut conn)
                .await?
                .grouped_by(&members.iter().map(|(_, u)| u).collect::<Vec<_>>())
                .iter()
                .zip(members.iter())
                .map(|(d, (m, u))| PrivateLeaderboardMember {
                    id: u.id,
                    username: u.username.clone(),
                    admin: m.admin,
                    time_coded: d.iter().map(|a| a.duration).sum(),
                })
                .collect::<Vec<_>>();

        Ok(PrivateLeaderboard {
            name: board.name,
            invite: board.invite_code,
            creation_time: board.creation_time,
            members,
        })
    }

    pub async fn add_user_to_leaderboard(
        &self,
        uid: i32,
        invite: String,
    ) -> Result<crate::api::users::MinimalLeaderboard, TimeError> {
        use crate::schema::leaderboards::dsl::{invite_code, leaderboards};

        let mut conn = self.db.get().await?;

        let board = leaderboards
            .filter(invite_code.eq(invite))
            .first::<Leaderboard>(&mut conn)
            .await?;

        let user = NewLeaderboardMember {
            user_id: uid,
            leaderboard_id: board.id,
            admin: false,
        };

        let name = board.name.clone();

        // FIXME: I'm not sure if a transaction is required here
        let member_count = conn
            .build_transaction()
            .read_write()
            .run(|mut conn| {
                Box::pin(async move {
                    diesel::insert_into(leaderboard_members::table)
                        .values(&user)
                        .execute(&mut conn)
                        .await?;

                    Ok::<i64, TimeError>(
                        LeaderboardMember::belonging_to(&board)
                            .count()
                            .first::<i64>(&mut conn)
                            .await?,
                    )
                })
            })
            .await?;

        Ok(crate::api::users::MinimalLeaderboard {
            name,
            member_count: member_count as i32,
        })
    }

    pub async fn remove_user_from_leaderboard(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;

        let mut conn = self.db.get().await?;

        Ok(diesel::delete(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .execute(&mut conn)
            .await?
            != 0)
    }

    pub async fn promote_user_to_leaderboard_admin(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;

        let mut conn = self.db.get().await?;
        Ok(diesel::update(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .set(admin.eq(true))
            .execute(&mut conn)
            .await?
            != 0)
    }

    pub async fn demote_user_to_leaderboard_member(
        &self,
        lid: i32,
        uid: i32,
    ) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;

        let mut conn = self.db.get().await?;
        Ok(diesel::update(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .set(admin.eq(false))
            .execute(&mut conn)
            .await?
            != 0)
    }

    pub async fn is_leaderboard_member(&self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::leaderboard_members::dsl::*;

        Ok(leaderboard_members
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .select(id)
            .first::<i32>(&mut conn)
            .await
            .optional()?
            .is_some())
    }

    pub async fn is_leaderboard_admin(&self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let mut conn = self.db.get().await?;

        Ok(leaderboard_members
            .filter(leaderboard_id.eq(lid).and(user_id.eq(uid)))
            .select(admin)
            .first::<bool>(&mut conn)
            .await
            .optional()?
            .unwrap_or(false))
    }

    pub async fn get_leaderboard_admin_count(&self, lid: i32) -> Result<i64, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let mut conn = self.db.get().await?;

        Ok(leaderboard_members
            .filter(leaderboard_id.eq(lid).and(admin.eq(true)))
            .select(diesel::dsl::count(user_id))
            .first::<i64>(&mut conn)
            .await?)
    }

    pub async fn get_user_leaderboards(
        &self,
        uid: i32,
    ) -> Result<Vec<crate::api::users::ListLeaderboard>, TimeError> {
        let mut conn = self.db.get().await?;

        let user = user_identities::table
            .find(uid)
            .first::<UserIdentity>(&mut conn)
            .await?;

        let boards = LeaderboardMember::belonging_to(&user)
            .inner_join(crate::schema::leaderboards::table)
            .select(crate::schema::leaderboards::dsl::leaderboards::all_columns())
            .load::<Leaderboard>(&mut conn)
            .await?;

        let aweekago = NaiveDateTime::new(
            Local::now().date_naive() - chrono::Duration::weeks(1),
            chrono::NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap(),
        );

        // FIXME: We could maybe use limit here because we only need the
        // user and the top member
        let members = LeaderboardMember::belonging_to(&boards.iter().collect::<Vec<_>>())
            //.inner_join(crate::schema::user_identities::table)
            .left_join(
                crate::schema::coding_activities::table.on(leaderboard_members::dsl::user_id
                    .eq(coding_activities::dsl::user_id)
                    .and(coding_activities::dsl::start_time.ge(aweekago))),
            )
            .group_by(leaderboard_members::dsl::id)
            .select((
                leaderboard_members::dsl::leaderboard_members::all_columns(),
                // NOTE: This is a work-around (diesel has some issues with group_by)
                diesel::dsl::sql::<diesel::sql_types::BigInt>(
                    "COALESCE(SUM(coding_activities.duration), 0) AS coding_time",
                ),
            ))
            .order_by(diesel::dsl::sql::<diesel::sql_types::BigInt>("coding_time").desc())
            .load::<(LeaderboardMember, i64)>(&mut conn)
            .await?;

        let usernames = LeaderboardMember::belonging_to(&boards.iter().collect::<Vec<_>>())
            .inner_join(crate::schema::user_identities::table)
            .group_by(user_identities::dsl::id)
            .select((user_identities::dsl::id, user_identities::dsl::username))
            .load_stream::<(i32, String)>(&mut conn)
            .await?
            .try_fold(HashMap::new(), |mut acc, (id, name)| {
                acc.insert(id, name);
                futures_util::future::ready(Ok(acc))
            })
            .await?;

        let members_populated = members
            .iter()
            .map(|(m, t)| {
                (
                    m.leaderboard_id,
                    PrivateLeaderboardMember {
                        id: m.user_id,
                        username: usernames[&m.user_id].clone(),
                        admin: m.admin,
                        time_coded: *t as i32,
                    },
                )
            })
            .collect::<Vec<_>>();

        Ok(boards
            .iter()
            .map(|b| {
                let bms = members_populated
                    .iter()
                    .filter(|(lid, _)| *lid == b.id)
                    .map(|(_, m)| m)
                    .collect::<Vec<_>>();

                let (my_position, me) = bms
                    .iter()
                    .enumerate()
                    .find(|(_, m)| m.id == user.id)
                    .expect("bug: impossible");
                ListLeaderboard {
                    name: b.name.clone(),
                    member_count: bms.len() as i32,
                    // NOTE: Sorted in the query
                    top_member: bms[0].clone(),
                    my_position: my_position as i32 + 1,
                    me: (*me).clone(),
                }
            })
            .collect::<Vec<_>>())
    }
}
