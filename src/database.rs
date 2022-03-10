use chrono::Duration;
use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::prelude::*;

use crate::{
    api::{DataRequest, HeartBeat},
    models::*,
    user::UserId,
};

impl Database {
    pub fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }

    pub fn get_user_by_discord_id(
        &self,
        discord_user_id: u64,
    ) -> Result<Option<RegisteredUser>, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(discord_id.eq(discord_user_id))
            .first::<RegisteredUser>(&self.pool.get()?)
            .optional()?)
    }

    fn user_exists(&self, username: &str) -> Result<bool, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .first::<RegisteredUser>(&self.pool.get()?)
            .optional()?
            .is_some())
    }

    pub fn get_user_by_name(&self, username: &str) -> Result<RegisteredUser, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .first::<RegisteredUser>(&self.pool.get()?)?)
    }

    fn get_user_hash_and_salt(&self, username: &str) -> Result<(Vec<u8>, Vec<u8>), anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers
            .filter(user_name.eq(username))
            .select((password, salt))
            .first::<(Vec<u8>, Vec<u8>)>(&self.pool.get()?)?)
    }

    pub fn verify_user_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<bool, anyhow::Error> {
        let (hash, salt) = self.get_user_hash_and_salt(username)?;
        let argon2 = Argon2::default();
        let salt = SaltString::new(std::str::from_utf8(salt.as_slice())?).unwrap();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        return Ok(password_hash.hash.unwrap().as_bytes() == hash);
    }

    pub fn new_user(&self, username: &str, password: &str) -> Result<String, anyhow::Error> {
        if self.user_exists(username)? {
            return Err(anyhow::anyhow!("User exists"));
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let token = crate::utils::generate_token();
        let hash = password_hash.hash.unwrap();
        let new_user = NewRegisteredUser {
            discord_id: 0,
            auth_token: token.clone(),
            registration_time: chrono::Local::now().naive_local(),
            user_name: username.to_string(),
            password: hash.as_bytes(),
            salt: dbg!(salt.as_bytes()),
        };
        diesel::insert_into(crate::schema::RegisteredUsers::table)
            .values(&new_user)
            .execute(&self.pool.get()?)?;
        Ok(token)
    }

    pub fn get_user_by_token(&self, token: &str) -> Result<UserId, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        let user = RegisteredUsers
            .select(id)
            .filter(auth_token.eq(token))
            .first::<i32>(&self.pool.get()?)?;
        Ok(UserId(user))
    }

    pub fn add_activity(
        &self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), anyhow::Error> {
        use crate::schema::CodingActivities::dsl::*;
        let activity = NewCodingActivity {
            user_id: updated_user_id,
            start_time: ctx_start_time,
            duration: ctx_duration.num_seconds() as i32,
            project_name: heartbeat.project_name,
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
        user: UserId,
    ) -> Result<Vec<CodingActivity>, anyhow::Error> {
        use crate::schema::CodingActivities::dsl::*;
        let mut query = CodingActivities.into_boxed().filter(user_id.eq(user.0));
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
}
