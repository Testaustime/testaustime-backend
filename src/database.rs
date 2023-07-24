use std::{future::Future, pin::Pin, sync::Arc};

use actix_web::{
    dev::Payload,
    web::{block, Data},
    FromRequest, HttpRequest,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{prelude::*, Duration};
use diesel::{
    insert_into,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};
use futures_util::future::join_all;

use crate::{
    error::TimeError,
    models::*,
    requests::{DataRequest, HeartBeat},
    utils::*,
};

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

impl DatabaseWrapper {
    pub async fn user_exists(&self, target_username: String) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .filter(username.eq(target_username))
                .first::<UserIdentity>(&mut conn)
                .optional()?
                .is_some())
        })
        .await
    }

    pub async fn get_user_by_name(
        &self,
        target_username: String,
    ) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .filter(lower(username).eq(target_username.to_lowercase()))
                .first::<UserIdentity>(&mut conn)?)
        })
        .await
    }

    pub async fn delete_user(&self, userid: i32) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(diesel::delete(user_identities.filter(id.eq(userid))).execute(&mut conn)? > 0)
        })
        .await
    }

    pub async fn get_user_by_id(&self, userid: i32) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .filter(id.eq(userid))
                .first::<UserIdentity>(&mut conn)?)
        })
        .await
    }

    pub async fn verify_user_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let user = self.get_user_by_name(username.to_string()).await?;
        let tuser = self.get_testaustime_user_by_id(user.id).await?;

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

    pub async fn regenerate_token(&self, userid: i32) -> Result<String, TimeError> {
        let token = crate::utils::generate_token();

        let token_clone = token.clone();

        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;

            diesel::update(crate::schema::user_identities::table)
                .filter(id.eq(userid))
                .set(auth_token.eq(token_clone))
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(token)
    }

    pub async fn new_testaustime_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<NewUserIdentity, TimeError> {
        use crate::schema::{testaustime_users, user_identities};
        if self.user_exists(username.to_string()).await? {
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

        let new_user_clone = new_user.clone();

        self.run_async_query(move |mut conn| {
            let id = diesel::insert_into(crate::schema::user_identities::table)
                .values(new_user_clone)
                .returning(user_identities::id)
                .get_results::<i32>(&mut conn)
                .map_err(|_| TimeError::UserExists)?;

            let testaustime_user = NewTestaustimeUser {
                password: hash.as_bytes().to_vec(),
                salt: salt.as_bytes().to_vec(),
                identity: id[0],
            };

            diesel::insert_into(testaustime_users::table)
                .values(&testaustime_user)
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(new_user)
    }

    pub async fn change_username(&self, user: i32, new_username: String) -> Result<(), TimeError> {
        if self.user_exists(new_username.to_string()).await? {
            return Err(TimeError::UserExists);
        }

        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;
            diesel::update(crate::schema::user_identities::table)
                .filter(id.eq(user))
                .set(username.eq(new_username))
                .execute(&mut conn)
                .map_err(|_| TimeError::UserExists)?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn change_password(&self, user: i32, new_password: &str) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .unwrap();
        let new_hash = password_hash.hash.unwrap();

        self.run_async_query(move |mut conn| {
            use crate::schema::testaustime_users::dsl::*;
            diesel::update(crate::schema::testaustime_users::table)
                .filter(identity.eq(user))
                .set((
                    password.eq(&new_hash.as_bytes()),
                    salt.eq(new_salt.as_bytes()),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn get_user_by_token(&self, token: String) -> Result<UserIdentity, TimeError> {
        let user = self
            .run_async_query(move |mut conn| {
                use crate::schema::user_identities::dsl::*;

                Ok(user_identities
                    .filter(auth_token.eq(token))
                    .first::<UserIdentity>(&mut conn)?)
            })
            .await?;

        Ok(user)
    }

    pub async fn add_activity(
        &self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), TimeError> {
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

        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;

            diesel::insert_into(coding_activities)
                .values(activity)
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn get_all_activity(&self, user: i32) -> Result<Vec<CodingActivity>, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;
            Ok(coding_activities
                .filter(user_id.eq(user))
                .load::<CodingActivity>(&mut conn)?)
        })
        .await
    }

    pub async fn get_activity(
        &self,
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

        let res = self
            .run_async_query(move |mut conn| Ok(query.load::<CodingActivity>(&mut conn)?))
            .await;

        res
    }

    pub async fn add_friend(&self, user: i32, friend: String) -> Result<UserIdentity, TimeError> {
        let Some(friend) = self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;

            Ok(user_identities
                .filter(friend_code.eq(friend))
                .first::<UserIdentity>(&mut conn)
                .optional()?)
        }).await? else {
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
                            .filter(id.eq(cur_friend))
                            .first::<UserIdentity>(&mut conn)
                            .ok()
                    })
                    .collect())
            })
            .await?;

        Ok(friends)
    }

    pub async fn get_friends_with_time(&mut self, user: i32) -> Result<Vec<FriendWithTime>, TimeError> {
        use crate::schema::{
            friend_relations::dsl::{friend_relations, greater_id, lesser_id},
            user_identities::dsl::*,
        };

        let friends = join_all(self.run_async_query(move |conn| {
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
            )})
            .await
            .map(|cur_friend| async {
                join_all(self.run_async_query(move |conn| {
                    Ok(user_identities
                       .filter(id.eq(cur_friend))
                       .first::<UserIdentity>(&mut conn)
                       .ok())
                })
                    .await
                    .map(|friends| {
                        friends.map(|friend| async { FriendWithTime {
                            coding_time: self.get_coding_time_steps(friend.id).await,
                            user: friend,
                        }})
                    }))
            }))
        .await?
        .collect()

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
            diesel::update(crate::schema::user_identities::table)
                .filter(id.eq(userid))
                .set(friend_code.eq(code_clone))
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(code)
    }

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
    ) -> Result<crate::api::users::ListLeaderboard, TimeError> {
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

        Ok(crate::api::users::ListLeaderboard { name, member_count })
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

    pub async fn get_user_coding_time_since(
        &self,
        uid: i32,
        since: chrono::NaiveDateTime,
    ) -> Result<i32, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;

            Ok(coding_activities
                .filter(user_id.eq(uid).and(start_time.ge(since)))
                .select(diesel::dsl::sum(duration))
                .first::<Option<i64>>(&mut conn)?
                .unwrap_or(0) as i32)
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

        let (names, memcount) = self
            .run_async_query(move |mut conn| {
                let n = {
                    use crate::schema::leaderboards::dsl::*;
                    leaderboards
                        .filter(id.eq_any(&ids))
                        .order_by(id.asc())
                        .select(name)
                        .load::<String>(&mut conn)?
                };
                let mut c = Vec::new();
                // FIXME: Do this in the query
                for i in ids {
                    c.push({
                        use crate::schema::leaderboard_members::dsl::*;
                        leaderboard_members
                            .filter(leaderboard_id.eq(i))
                            .select(diesel::dsl::count(user_id))
                            .first::<i64>(&mut conn)? as i32
                    })
                }

                Ok((n, c))
            })
            .await?;
        let mut ret = Vec::new();
        for (n, c) in names.iter().zip(memcount) {
            ret.push(crate::api::users::ListLeaderboard {
                name: n.to_string(),
                member_count: c,
            });
        }
        Ok(ret)
    }

    pub async fn get_coding_time_steps(&self, uid: i32) -> CodingTimeSteps {
        CodingTimeSteps {
            all_time: self
                .get_user_coding_time_since(uid, chrono::NaiveDateTime::from_timestamp(0, 0))
                .await
                .unwrap_or(0),
            past_month: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(30),
                )
                .await
                .unwrap_or(0),
            past_week: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(7),
                )
                .await
                .unwrap_or(0),
        }
    }

    pub async fn get_testaustime_user_by_id(&self, uid: i32) -> Result<TestaustimeUser, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::testaustime_users::dsl::*;

            Ok(testaustime_users
                .filter(identity.eq(uid))
                .first::<TestaustimeUser>(&mut conn)?)
        })
        .await
    }

    #[cfg(feature = "testausid")]
    pub async fn testausid_login(
        &self,
        user_id_arg: String,
        username: String,
        platform_id: String,
    ) -> Result<String, TimeError> {
        use crate::schema::{
            testausid_users::dsl::{identity, testausid_users, user_id},
            user_identities::dsl::{auth_token, id, user_identities},
        };

        let user_id_arg_clone = user_id_arg.clone();

        let user_identity_opt = self
            .run_async_query(move |mut conn| {
                Ok(testausid_users
                    .filter(user_id.eq(user_id_arg_clone))
                    .select(identity)
                    .first::<i32>(&mut conn)
                    .optional()?)
            })
            .await?;

        if let Some(user_identity) = user_identity_opt {
            let token = self
                .run_async_query(move |mut conn| {
                    Ok(user_identities
                        .filter(id.eq(user_identity))
                        .select(auth_token)
                        .first::<String>(&mut conn)?)
                })
                .await?;

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
            let new_user_id = self
                .run_async_query(move |mut conn| {
                    Ok(diesel::insert_into(crate::schema::user_identities::table)
                        .values(&new_user)
                        .returning(id)
                        .get_results::<i32>(&mut conn)
                        .map_err(|_| TimeError::UserExists)?)
                })
                .await?;

            let testausid_user = NewTestausIdUser {
                user_id: user_id_arg,
                identity: new_user_id[0],
                service_id: platform_id,
            };

            self.run_async_query(move |mut conn| {
                diesel::insert_into(testausid_users)
                    .values(&testausid_user)
                    .execute(&mut conn)?;

                Ok(())
            })
            .await?;

            Ok(token)
        }
    }

    pub async fn change_visibility(&self, userid: i32, visibility: bool) -> Result<(), TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;
            diesel::update(user_identities.filter(id.eq(userid)))
                .set(is_public.eq(visibility))
                .execute(&mut conn)?;
            Ok(())
        })
        .await
    }

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

    pub async fn rename_project(
        &self,
        target_user_id: i32,
        from: String,
        to: String,
    ) -> Result<usize, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;
            Ok(diesel::update(coding_activities)
                .filter(user_id.eq(target_user_id))
                .filter(project_name.eq(from))
                .set(project_name.eq(to))
                .execute(&mut conn)?)
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
