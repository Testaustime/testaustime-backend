use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{prelude::*, Duration};
use diesel::{insert_into, prelude::*, r2d2::{ConnectionManager, Pool}};

use crate::{
    error::TimeError,
    models::*,
    requests::{DataRequest, HeartBeat},
    utils::*,
    DbConnection,
};

pub struct Database {
    pub backend: Box<dyn DatabaseConnectionPool>,
}

impl Database {
    pub fn get(&self) -> Result<Box<dyn DatabaseConnection>, TimeError> {
        Ok(self.backend.get()?)
    }
}

pub trait DatabaseConnectionPool: Send + Sync {
    fn get(&self) -> Result<Box<dyn DatabaseConnection>, TimeError>;
}

impl DatabaseConnectionPool for Pool<ConnectionManager<PgConnection>> {
    fn get(&self) -> Result<Box<dyn DatabaseConnection>, TimeError> {
        Ok(Box::new(self.get()?))
    }
}

pub trait DatabaseConnection: Send  {
    fn user_exists(&mut self, target_username: &str) -> Result<bool, TimeError>;

    fn get_user_by_name(&mut self, target_username: &str) -> Result<UserIdentity, TimeError>;

    fn delete_user(&mut self, userid: i32) -> Result<bool, TimeError>;

    fn get_user_by_id(&mut self, userid: i32) -> Result<UserIdentity, TimeError>;

    fn verify_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError>;

    fn regenerate_token(&mut self, userid: i32) -> Result<String, TimeError>;

    fn new_testaustime_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<NewUserIdentity, TimeError>;

    fn change_username(&mut self, user: i32, new_username: &str) -> Result<(), TimeError>;

    fn change_password(&mut self, user: i32, new_password: &str) -> Result<(), TimeError>;

    fn get_user_by_token(&mut self, token: &str) -> Result<UserIdentity, TimeError>;

    fn add_activity(
        &mut self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), TimeError>;

    fn get_all_activity(&mut self, user: i32) -> Result<Vec<CodingActivity>, TimeError>;

    fn get_activity(
        &mut self,
        request: DataRequest,
        user: i32,
    ) -> Result<Vec<CodingActivity>, TimeError>;

    fn add_friend(&mut self, user: i32, friend: &str) -> Result<UserIdentity, TimeError>;

    fn get_friends(&mut self, user: i32) -> Result<Vec<UserIdentity>, TimeError>;

    fn are_friends(&mut self, user: i32, friend_id: i32) -> Result<bool, TimeError>;

    fn remove_friend(&mut self, user: i32, friend_id: i32) -> Result<bool, TimeError>;

    fn regenerate_friend_code(&mut self, userid: i32) -> Result<String, TimeError>;

    fn delete_activity(&mut self, userid: i32, activity: i32) -> Result<bool, TimeError>;

    fn create_leaderboard(&mut self, creator_id: i32, name: &str) -> Result<String, TimeError>;

    fn regenerate_leaderboard_invite(&mut self, lid: i32) -> Result<String, TimeError>;

    fn delete_leaderboard(&mut self, lname: &str) -> Result<bool, TimeError>;

    fn get_leaderboard_id_by_name(&mut self, lname: &str) -> Result<i32, TimeError>;

    fn get_leaderboard(&mut self, lname: &str) -> Result<PrivateLeaderboard, TimeError>;

    fn add_user_to_leaderboard(
        &mut self,
        uid: i32,
        invite: &str,
    ) -> Result<crate::api::users::ListLeaderboard, TimeError>;

    fn remove_user_from_leaderboard(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError>;

    fn promote_user_to_leaderboard_admin(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError>;

    fn demote_user_to_leaderboard_member(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError>;

    fn is_leaderboard_member(&mut self, uid: i32, lid: i32) -> Result<bool, TimeError>;

    fn get_user_coding_time_since(
        &mut self,
        uid: i32,
        since: chrono::NaiveDateTime,
    ) -> Result<i32, TimeError>;

    fn is_leaderboard_admin(&mut self, uid: i32, lid: i32) -> Result<bool, TimeError>;

    fn get_leaderboard_admin_count(&mut self, lid: i32) -> Result<i64, TimeError>;

    fn get_user_leaderboards(
        &mut self,
        uid: i32,
    ) -> Result<Vec<crate::api::users::ListLeaderboard>, TimeError>;

    fn get_coding_time_steps(&mut self, uid: i32) -> CodingTimeSteps;

    fn get_testaustime_user_by_id(&mut self, uid: i32) -> Result<TestaustimeUser, TimeError>;

    #[cfg(feature = "testausid")]
    fn testausid_login(
        &mut self,
        user_id_arg: String,
        username: String,
        platform_id: String,
    ) -> Result<String, TimeError>;

    fn change_visibility(&mut self, userid: i32, visibility: bool) -> Result<(), TimeError>;

    fn search_public_users(&mut self, search: &str) -> Result<Vec<PublicUser>, TimeError>;

    fn rename_project(&mut self, user_id: i32, from: &str, to: &str) -> Result<usize, TimeError>;
}

impl DatabaseConnection for DbConnection {
    fn user_exists(&mut self, target_username: &str) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;
        Ok(user_identities
            .filter(username.eq(target_username))
            .first::<UserIdentity>(self)
            .optional()?
            .is_some())
    }

    fn get_user_by_name(&mut self, target_username: &str) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

        Ok(user_identities
            .filter(lower(username).eq(target_username.to_lowercase()))
            .first::<UserIdentity>(self)?)
    }

    fn delete_user(&mut self, userid: i32) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;
        Ok(diesel::delete(user_identities.filter(id.eq(userid))).execute(self)? > 0)
    }

    fn get_user_by_id(&mut self, userid: i32) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        Ok(user_identities
            .filter(id.eq(userid))
            .first::<UserIdentity>(self)?)
    }

    fn verify_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let user = self.get_user_by_name(username)?;
        let tuser = self.get_testaustime_user_by_id(user.id)?;

        let argon2 = Argon2::default();
        let Ok(salt) = SaltString::new(std::str::from_utf8(&tuser.salt).unwrap()) else {
        return Ok(None); // The user has no password
    };
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        if password_hash.hash.unwrap().as_bytes() == tuser.password {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    fn regenerate_token(&mut self, userid: i32) -> Result<String, TimeError> {
        let token = crate::utils::generate_token();
        use crate::schema::user_identities::dsl::*;
        diesel::update(crate::schema::user_identities::table)
            .filter(id.eq(userid))
            .set(auth_token.eq(&token))
            .execute(self)?;
        Ok(token)
    }

    fn new_testaustime_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<NewUserIdentity, TimeError> {
        use crate::schema::{testaustime_users, user_identities};
        if self.user_exists(username)? {
            return Err(TimeError::UserExists);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let token = generate_token();
        let hash = password_hash.hash.unwrap();
        let new_user = NewUserIdentity {
            auth_token: token,
            registration_time: chrono::Local::now().naive_local(),
            username: username.to_string(),
            friend_code: generate_friend_code(),
        };
        let id = diesel::insert_into(crate::schema::user_identities::table)
            .values(&new_user)
            .returning(user_identities::id)
            .get_results::<i32>(self)
            .map_err(|_| TimeError::UserExists)?;

        let testaustime_user = NewTestaustimeUser {
            password: hash.as_bytes().to_vec(),
            salt: salt.as_bytes().to_vec(),
            identity: id[0],
        };

        diesel::insert_into(testaustime_users::table)
            .values(&testaustime_user)
            .execute(self)?;

        Ok(new_user)
    }

    fn change_username(&mut self, user: i32, new_username: &str) -> Result<(), TimeError> {
        if self.user_exists(new_username)? {
            return Err(TimeError::UserExists);
        }
        use crate::schema::user_identities::dsl::*;
        diesel::update(crate::schema::user_identities::table)
            .filter(id.eq(user))
            .set(username.eq(new_username))
            .execute(self)
            .map_err(|_| TimeError::UserExists)?;
        Ok(())
    }

    fn change_password(&mut self, user: i32, new_password: &str) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .unwrap();
        let new_hash = password_hash.hash.unwrap();
        use crate::schema::testaustime_users::dsl::*;
        diesel::update(crate::schema::testaustime_users::table)
            .filter(identity.eq(user))
            .set((
                password.eq(&new_hash.as_bytes()),
                salt.eq(new_salt.as_bytes()),
            ))
            .execute(self)?;
        Ok(())
    }

    fn get_user_by_token(&mut self, token: &str) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        let user = user_identities
            .filter(auth_token.eq(token))
            .first::<UserIdentity>(self)?;
        Ok(user)
    }

    fn add_activity(
        &mut self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), TimeError> {
        use crate::schema::coding_activities::dsl::*;
        let activity = NewCodingActivity {
            user_id: updated_user_id,
            start_time: ctx_start_time,
            duration: ctx_duration.num_seconds() as i32,
            project_name: if heartbeat.project_name.is_some()
                && heartbeat.project_name.as_ref().unwrap().starts_with("tmp.")
            {
                Some(String::from("tmp"))
            } else {
                heartbeat.project_name.map(|s| s.to_lowercase())
            },
            language: heartbeat.language,
            editor_name: heartbeat.editor_name,
            hostname: heartbeat.hostname,
        };
        diesel::insert_into(coding_activities)
            .values(activity)
            .execute(self)?;
        Ok(())
    }

    fn get_all_activity(&mut self, user: i32) -> Result<Vec<CodingActivity>, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        Ok(coding_activities
            .filter(user_id.eq(user))
            .load::<CodingActivity>(self)?)
    }

    fn get_activity(
        &mut self,
        request: DataRequest,
        user: i32,
    ) -> Result<Vec<CodingActivity>, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        let mut query = coding_activities.into_boxed().filter(user_id.eq(user));
        if let Some(from) = request.from {
            query = query.filter(start_time.ge(from.naive_local()));
        };
        if let Some(to) = request.to {
            query = query.filter(start_time.le(to.naive_local()));
        };
        if let Some(editor) = request.editor_name {
            query = query.filter(editor_name.eq(editor));
        };
        if let Some(project) = request.project_name {
            query = query.filter(project_name.eq(project));
        };
        if let Some(request_hostname) = request.hostname {
            query = query.filter(hostname.eq(request_hostname));
        };
        if let Some(request_language) = request.language {
            query = query.filter(language.eq(request_language));
        };
        if let Some(min_duration) = request.min_duration {
            query = query.filter(duration.ge(min_duration));
        };
        let res = query.load::<CodingActivity>(self)?;
        Ok(res)
    }

    fn add_friend(&mut self, user: i32, friend: &str) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        let Some(friend) = user_identities
        .filter(friend_code.eq(friend))
        .first::<UserIdentity>(self)
        .optional()?
    else {
        return Err(TimeError::UserNotFound)
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
            .execute(self)?;
        Ok(friend)
    }

    fn get_friends(&mut self, user: i32) -> Result<Vec<UserIdentity>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };
        let friends = friend_relations
            .filter(greater_id.eq(user).or(lesser_id.eq(user)))
            .load::<FriendRelation>(self)?
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
                    .filter(id.eq(cur_friend))
                    .first::<UserIdentity>(self)
                    .ok()
            })
            .collect();
        Ok(friends)
    }

    fn are_friends(&mut self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };
        Ok(friend_relations
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .first::<FriendRelation>(self)
            .optional()?
            .is_some())
    }

    fn remove_friend(&mut self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::friend_relations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };
        Ok(diesel::delete(friend_relations)
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .execute(self)?
            != 0)
    }

    fn regenerate_friend_code(&mut self, userid: i32) -> Result<String, TimeError> {
        use crate::schema::user_identities::dsl::*;
        let code = crate::utils::generate_friend_code();
        diesel::update(crate::schema::user_identities::table)
            .filter(id.eq(userid))
            .set(friend_code.eq(&code))
            .execute(self)?;
        Ok(code)
    }

    fn delete_activity(&mut self, userid: i32, activity: i32) -> Result<bool, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        let res = diesel::delete(crate::schema::coding_activities::table)
            .filter(id.eq(activity))
            .filter(user_id.eq(userid))
            .execute(self)?;
        Ok(res != 0)
    }

    fn create_leaderboard(&mut self, creator_id: i32, name: &str) -> Result<String, TimeError> {
        let code = crate::utils::generate_token();
        let board = NewLeaderboard {
            name: name.to_string(),
            creation_time: chrono::Local::now().naive_local(),
            invite_code: code.clone(),
        };
        let lid = {
            use crate::schema::leaderboards::dsl::*;
            insert_into(crate::schema::leaderboards::table)
                .values(&board)
                .returning(id)
                .get_results(self)?[0]
        };

        let admin = NewLeaderboardMember {
            user_id: creator_id,
            admin: true,
            leaderboard_id: lid,
        };
        insert_into(crate::schema::leaderboard_members::table)
            .values(&admin)
            .execute(self)?;
        Ok(code)
    }

    fn regenerate_leaderboard_invite(&mut self, lid: i32) -> Result<String, TimeError> {
        let newinvite = crate::utils::generate_token();
        use crate::schema::leaderboards::dsl::*;
        diesel::update(crate::schema::leaderboards::table)
            .filter(id.eq(lid))
            .set(invite_code.eq(&newinvite))
            .execute(self)?;
        Ok(newinvite)
    }

    fn delete_leaderboard(&mut self, lname: &str) -> Result<bool, TimeError> {
        use crate::schema::leaderboards::dsl::*;
        let res = diesel::delete(crate::schema::leaderboards::table)
            .filter(name.eq(lname))
            .execute(self)?;
        Ok(res != 0)
    }

    fn get_leaderboard_id_by_name(&mut self, lname: &str) -> Result<i32, TimeError> {
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
        use crate::schema::leaderboards::dsl::*;
        Ok(leaderboards
            .filter(lower(name).eq(lname.to_lowercase()))
            .select(id)
            .first::<i32>(self)?)
    }

    fn get_leaderboard(&mut self, lname: &str) -> Result<PrivateLeaderboard, TimeError> {
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);
        let board = {
            use crate::schema::leaderboards::dsl::*;
            leaderboards
                .filter(lower(name).eq(lname.to_lowercase()))
                .first::<Leaderboard>(self)?
        };
        let members = {
            use crate::schema::leaderboard_members::dsl::*;
            leaderboard_members
                .filter(leaderboard_id.eq(board.id))
                .load::<LeaderboardMember>(self)?
        };
        let mut fullmembers = Vec::new();
        let aweekago = NaiveDateTime::new(
            chrono::Local::today().naive_local() - chrono::Duration::weeks(1),
            chrono::NaiveTime::from_num_seconds_from_midnight(0, 0),
        );
        for m in members {
            if let Ok(user) = self.get_user_by_id(m.user_id) {
                fullmembers.push(PrivateLeaderboardMember {
                    username: user.username,
                    admin: m.admin,
                    time_coded: self
                        .get_user_coding_time_since(m.user_id, aweekago)
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

    fn add_user_to_leaderboard(
        &mut self,
        uid: i32,
        invite: &str,
    ) -> Result<crate::api::users::ListLeaderboard, TimeError> {
        let (lid, name) = {
            use crate::schema::leaderboards::dsl::*;
            leaderboards
                .filter(invite_code.eq(invite))
                .select((id, name))
                .first::<(i32, String)>(self)?
        };
        let user = NewLeaderboardMember {
            user_id: uid,
            leaderboard_id: lid,
            admin: false,
        };
        insert_into(crate::schema::leaderboard_members::table)
            .values(&user)
            .execute(self)?;
        let member_count: i32 = {
            use crate::schema::leaderboard_members::dsl::*;
            leaderboard_members
                .filter(leaderboard_id.eq(lid))
                .select(diesel::dsl::count(user_id))
                .first::<i64>(self)? as i32
        };
        Ok(crate::api::users::ListLeaderboard { name, member_count })
    }

    fn remove_user_from_leaderboard(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let res = diesel::delete(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .execute(self)?;
        Ok(res != 0)
    }

    fn promote_user_to_leaderboard_admin(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let res = diesel::update(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .set(admin.eq(true))
            .execute(self)?;
        Ok(res != 0)
    }

    fn demote_user_to_leaderboard_member(&mut self, lid: i32, uid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        let res = diesel::update(crate::schema::leaderboard_members::table)
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .set(admin.eq(false))
            .execute(self)?;
        Ok(res != 0)
    }

    fn is_leaderboard_member(&mut self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        Ok(leaderboard_members
            .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
            .select(id)
            .first::<i32>(self)
            .optional()?
            .is_some())
    }

    fn get_user_coding_time_since(
        &mut self,
        uid: i32,
        since: chrono::NaiveDateTime,
    ) -> Result<i32, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        Ok(coding_activities
            .filter(user_id.eq(uid).and(start_time.ge(since)))
            .select(diesel::dsl::sum(duration))
            .first::<Option<i64>>(self)?
            .unwrap_or(0) as i32)
    }

    fn is_leaderboard_admin(&mut self, uid: i32, lid: i32) -> Result<bool, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        Ok(leaderboard_members
            .filter(leaderboard_id.eq(lid).and(user_id.eq(uid)))
            .select(admin)
            .first::<bool>(self)
            .optional()?
            .unwrap_or(false))
    }

    fn get_leaderboard_admin_count(&mut self, lid: i32) -> Result<i64, TimeError> {
        use crate::schema::leaderboard_members::dsl::*;
        Ok(leaderboard_members
            .filter(leaderboard_id.eq(lid).and(admin.eq(true)))
            .select(diesel::dsl::count(user_id))
            .first::<i64>(self)?)
    }

    fn get_user_leaderboards(
        &mut self,
        uid: i32,
    ) -> Result<Vec<crate::api::users::ListLeaderboard>, TimeError> {
        let ids = {
            use crate::schema::leaderboard_members::dsl::*;
            leaderboard_members
                .filter(user_id.eq(uid))
                .select(leaderboard_id)
                .order_by(leaderboard_id.asc())
                .load::<i32>(self)?
        };
        let (names, memcount) = {
            let n = {
                use crate::schema::leaderboards::dsl::*;
                leaderboards
                    .filter(id.eq_any(&ids))
                    .order_by(id.asc())
                    .select(name)
                    .load::<String>(self)?
            };
            let mut c = Vec::new();
            // FIXME: Do this in the query
            for i in ids {
                c.push({
                    use crate::schema::leaderboard_members::dsl::*;
                    leaderboard_members
                        .filter(leaderboard_id.eq(i))
                        .select(diesel::dsl::count(user_id))
                        .first::<i64>(self)? as i32
                })
            }
            (n, c)
        };
        let mut ret = Vec::new();
        for (n, c) in names.iter().zip(memcount) {
            ret.push(crate::api::users::ListLeaderboard {
                name: n.to_string(),
                member_count: c,
            });
        }
        Ok(ret)
    }

    fn get_coding_time_steps(&mut self, uid: i32) -> CodingTimeSteps {
        CodingTimeSteps {
            all_time: self
                .get_user_coding_time_since(uid, chrono::NaiveDateTime::from_timestamp(0, 0))
                .unwrap_or(0),
            past_month: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(30),
                )
                .unwrap_or(0),
            past_week: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(7),
                )
                .unwrap_or(0),
        }
    }

    fn get_testaustime_user_by_id(&mut self, uid: i32) -> Result<TestaustimeUser, TimeError> {
        use crate::schema::testaustime_users::dsl::*;
        Ok(testaustime_users
            .filter(identity.eq(uid))
            .first::<TestaustimeUser>(self)?)
    }

    #[cfg(feature = "testausid")]
    fn testausid_login(
        &mut self,
        user_id_arg: String,
        username: String,
        platform_id: String,
    ) -> Result<String, TimeError> {
        use crate::schema::{
            testausid_users::dsl::{identity, testausid_users, user_id},
            user_identities::dsl::{auth_token, id, user_identities},
        };

        let user_identity_opt = testausid_users
            .filter(user_id.eq(&user_id_arg))
            .select(identity)
            .first::<i32>(self)
            .optional()?;

        if let Some(user_identity) = user_identity_opt {
            let token = user_identities
                .filter(id.eq(user_identity))
                .select(auth_token)
                .first::<String>(self)?;

            Ok(token)
        } else {
            let token = generate_token();
            let new_user = NewUserIdentity {
                //FIXME: You can get around using a clone here
                auth_token: token.clone(),
                registration_time: chrono::Local::now().naive_local(),
                username,
                friend_code: generate_friend_code(),
            };
            let new_user_id = diesel::insert_into(crate::schema::user_identities::table)
                .values(&new_user)
                .returning(id)
                .get_results::<i32>(self)
                .map_err(|_| TimeError::UserExists)?;

            let testausid_user = NewTestausIdUser {
                user_id: user_id_arg,
                identity: new_user_id[0],
                service_id: platform_id,
            };

            diesel::insert_into(testausid_users)
                .values(&testausid_user)
                .execute(self)?;

            Ok(token)
        }
    }

    fn change_visibility(&mut self, userid: i32, visibility: bool) -> Result<(), TimeError> {
        use crate::schema::user_identities::dsl::*;
        diesel::update(user_identities.filter(id.eq(userid)))
            .set(is_public.eq(visibility))
            .execute(self)?;
        Ok(())
    }

    fn search_public_users(&mut self, search: &str) -> Result<Vec<PublicUser>, TimeError> {
        use crate::schema::user_identities::dsl::*;
        Ok(user_identities
            .filter(is_public.eq(true))
            .filter(username.like(format!("%{}%", search)))
            .load::<UserIdentity>(self)?
            .into_iter()
            .map(|u| u.into())
            .collect())
    }

    fn rename_project(
        &mut self,
        target_user_id: i32,
        from: &str,
        to: &str,
    ) -> Result<usize, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        Ok(diesel::update(coding_activities)
            .filter(user_id.eq(target_user_id))
            .filter(project_name.eq(from))
            .set(project_name.eq(to))
            .execute(self)?)
    }
}
