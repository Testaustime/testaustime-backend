use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::{api::HeartBeat, models::*, user::User};

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

    pub fn new_user(&self, discord_user_id: u64) -> Result<RegisteredUser, anyhow::Error> {
        let new_user = NewRegisteredUser {
            discord_id: discord_user_id,
            auth_token: crate::utils::generate_token(),
            registration_time: chrono::Local::now().naive_local(),
            user_name: String::from(""),
        };
        diesel::insert_into(crate::schema::RegisteredUsers::table)
            .values(&new_user)
            .execute(&self.pool.get()?)?;

        self.get_user_by_discord_id(discord_user_id)?
            .ok_or(anyhow::anyhow!("No user found"))
    }

    pub fn get_user_by_token(&self, token: &str) -> Result<User, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        let user = RegisteredUsers
            .select(id)
            .filter(auth_token.eq(token))
            .first::<i32>(&self.pool.get()?)?;
        Ok(User(user))
    }

    // FIXME: Multiple sessions?
    pub fn update_activity(
        &self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
    ) -> Result<(), anyhow::Error> {
        use crate::schema::CodingActivities::dsl::*;
        let test = CodingActivities
            .filter(project_name.eq(&heartbeat.project_name).or(project_name.is_null()))
            .filter(language.eq(&heartbeat.language).or(language.is_null()))
            .filter(editor_name.eq(&heartbeat.editor_name).or(editor_name.is_null()))
            .filter(hostname.eq(&heartbeat.hostname).or(hostname.is_null()))
            .order_by(start_time.desc())
            .first::<CodingActivity>(&self.pool.get()?)
            .optional()?;
        match test {
            Some(test) => {
                diesel::update(CodingActivities)
                    .filter(project_name.eq(heartbeat.project_name).or(project_name.is_null()))
                    .filter(language.eq(heartbeat.language).or(language.is_null()))
                    .filter(editor_name.eq(heartbeat.editor_name).or(editor_name.is_null()))
                    .filter(hostname.eq(heartbeat.hostname).or(hostname.is_null()))
                    .set(duration.eq((Local::now().naive_local() - test.start_time).num_seconds() as i32))
                    .execute(&self.pool.get()?)?;
            },
            None => {
                let activity = NewCodingActivity {
                    user_id: updated_user_id,
                    start_time: Local::now().naive_local(),
                    duration: 0,
                    project_name: heartbeat.project_name,
                    language: heartbeat.language,
                    editor_name: heartbeat.editor_name,
                    hostname: heartbeat.hostname,
                };
                let user = diesel::insert_into(CodingActivities)
                    .values(activity)
                    .execute(&self.pool.get()?)?;
            }
        };
        Ok(())
    }
}
