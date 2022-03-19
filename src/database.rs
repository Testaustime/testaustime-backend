use chrono::Duration;
use diesel::{
    insert_into,
    mysql::MysqlConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

use crate::utils::*;

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::prelude::*;

use crate::{
    error::TimeError,
    models::*,
    requests::{DataRequest, HeartBeat},
    user::UserId,
};

impl Database {
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<MysqlConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }

    fn user_exists(&self, username: &str) -> Result<bool, TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .first::<RegisteredUser>(&self.pool.get()?)
            .optional()?
            .is_some())
    }

    pub fn get_user_by_name(&self, username: &str) -> Result<RegisteredUser, TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .first::<RegisteredUser>(&self.pool.get()?)?)
    }

    pub fn get_user_by_id(&self, userid: UserId) -> Result<RegisteredUser, TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(id.eq(userid.id))
            .first::<RegisteredUser>(&self.pool.get()?)?)
    }

    fn get_user_hash_and_salt(&self, username: &str) -> Result<(Vec<u8>, Vec<u8>), TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .select((password, salt))
            .first::<(Vec<u8>, Vec<u8>)>(&self.pool.get()?)?)
    }

    pub fn verify_user_password(&self, username: &str, password: &str) -> Result<bool, TimeError> {
        let (hash, salt) = self.get_user_hash_and_salt(username)?;
        let argon2 = Argon2::default();
        let Ok(salt) = SaltString::new(&String::from_utf8(salt).unwrap()) else {
            return Ok(false); // The user has no password
        };
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        return Ok(password_hash.hash.unwrap().as_bytes() == hash);
    }

    pub fn regenerate_token(&self, userid: UserId) -> Result<String, TimeError> {
        let token = crate::utils::generate_token();
        use crate::schema::RegisteredUsers::dsl::*;
        diesel::update(crate::schema::RegisteredUsers::table)
            .filter(id.eq(userid.id))
            .set(auth_token.eq(&token))
            .execute(&self.pool.get()?)?;
        Ok(token)
    }

    pub fn new_user(&self, username: &str, password: &str) -> Result<String, TimeError> {
        if self.user_exists(username)? {
            return Err(TimeError::UserExistsError);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let token = generate_token();
        let hash = password_hash.hash.unwrap();
        let new_user = NewRegisteredUser {
            auth_token: token.clone(),
            registration_time: chrono::Local::now().naive_local(),
            user_name: username.to_string(),
            friend_code: Some(generate_friend_code()),
            password: hash.as_bytes(),
            salt: salt.as_bytes(),
        };
        diesel::insert_into(crate::schema::RegisteredUsers::table)
            .values(&new_user)
            .execute(&self.pool.get()?)?;
        Ok(token)
    }

    pub fn change_user_password_to(
        &self,
        user: UserId,
        new_password: &str,
    ) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .unwrap();
        let new_hash = password_hash.hash.unwrap();
        use crate::schema::RegisteredUsers::dsl::*;
        diesel::update(crate::schema::RegisteredUsers::table)
            .filter(id.eq(user.id))
            .set((
                password.eq(&new_hash.as_bytes()),
                salt.eq(new_salt.as_bytes()),
            ))
            .execute(&self.pool.get()?)?;
        Ok(())
    }

    pub fn get_user_by_token(&self, token: &str) -> Result<UserId, TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        let user = RegisteredUsers
            .select(id)
            .filter(auth_token.eq(token))
            .first::<i32>(&self.pool.get()?)?;
        Ok(UserId { id: user })
    }

    pub fn add_activity(
        &self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), TimeError> {
        use crate::schema::CodingActivities::dsl::*;
        let activity = NewCodingActivity {
            user_id: updated_user_id,
            start_time: ctx_start_time,
            duration: ctx_duration.num_seconds() as i32,
            project_name: if heartbeat.project_name.is_some()
                && heartbeat.project_name.as_ref().unwrap().starts_with("tmp.")
            {
                Some(String::from("tmp"))
            } else {
                heartbeat.project_name
            },
            language: heartbeat.language,
            editor_name: heartbeat.editor_name,
            hostname: heartbeat.hostname,
        };
        diesel::insert_into(CodingActivities)
            .values(activity)
            .execute(&self.pool.get()?)?;
        Ok(())
    }

    pub fn get_activity(
        &self,
        request: DataRequest,
        user: i32,
    ) -> Result<Vec<CodingActivity>, TimeError> {
        use crate::schema::CodingActivities::dsl::*;
        let mut query = CodingActivities.into_boxed().filter(user_id.eq(user));
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
        let res = query.load::<CodingActivity>(&self.pool.get()?).unwrap();
        Ok(res)
    }

    pub fn add_friend(&self, user: UserId, friend: &str) -> Result<(), TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        let friend_id = RegisteredUsers
            .filter(friend_code.eq(friend))
            .select(id)
            .first::<i32>(&self.pool.get()?)
            .optional()?;

        if let Some(friend_id) = friend_id {
            let (lesser, greater) = if user.id < friend_id {
                (user.id, friend_id)
            } else {
                (friend_id, user.id)
            };
            // FIXME: Duplicates are not handled correctly, should not be an internal server error
            insert_into(crate::schema::FriendRelations::table)
                .values(crate::models::NewFriendRelation {
                    lesser_id: lesser,
                    greater_id: greater,
                })
                .execute(&self.pool.get()?)?;
        }
        Ok(())
    }

    pub fn get_friends(&self, user: UserId) -> Result<Vec<String>, TimeError> {
        use crate::schema::{
            FriendRelations::dsl::{greater_id, lesser_id, FriendRelations},
            RegisteredUsers::dsl::*,
        };
        let friends = FriendRelations
            .filter(greater_id.eq(user.id).or(lesser_id.eq(user.id)))
            .load::<FriendRelation>(&self.pool.get()?)?
            .iter()
            .map(
                |&FriendRelation {
                     lesser_id: other_lesser_id,
                     greater_id: other_greater_id,
                     ..
                 }| {
                    if other_lesser_id == user.id {
                        other_greater_id
                    } else {
                        other_lesser_id
                    }
                },
            )
            .filter_map(|cur_friend| {
                Some(
                    RegisteredUsers
                        .filter(id.eq(cur_friend))
                        .first::<RegisteredUser>(&self.pool.get().ok()?)
                        .ok()?
                        .user_name,
                )
            })
            .collect();
        Ok(friends)
    }

    pub fn are_friends(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::FriendRelations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };
        Ok(FriendRelations
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .first::<FriendRelation>(&self.pool.get()?)
            .optional()?
            .is_some())
    }

    pub fn remove_friend(&self, user: i32, friend_id: i32) -> Result<bool, TimeError> {
        use crate::schema::FriendRelations::dsl::*;
        let (lesser, greater) = if user < friend_id {
            (user, friend_id)
        } else {
            (friend_id, user)
        };
        Ok(diesel::delete(FriendRelations)
            .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
            .execute(&self.pool.get()?)?
            != 0)
    }

    pub fn regenerate_friend_code(&self, userid: UserId) -> Result<String, TimeError> {
        use crate::schema::RegisteredUsers::dsl::*;
        let code = crate::utils::generate_friend_code();
        diesel::update(crate::schema::RegisteredUsers::table)
            .filter(id.eq(userid.id))
            .set(friend_code.eq(&code))
            .execute(&self.pool.get()?)?;
        Ok(code)
    }

    pub fn delete_activity(&self, userid: i32, activity: i32) -> Result<bool, TimeError> {
        use crate::schema::CodingActivities::dsl::*;
        let res = diesel::delete(crate::schema::CodingActivities::table)
            .filter(id.eq(activity))
            .filter(user_id.eq(userid))
            .execute(&self.pool.get()?)?;
        Ok(res != 0)
    }
}
