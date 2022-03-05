use chrono::Duration;
use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

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

    pub fn new_user(&self, username: &str) -> Result<String, anyhow::Error> {
        let token = crate::utils::generate_token();
        let new_user = NewRegisteredUser {
            discord_id: 0,
            auth_token: token.clone(),
            registration_time: chrono::Local::now().naive_local(),
            user_name: String::from(""),
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
        let res = query.load::<CodingActivity>(&self.pool.get()?).unwrap();
        Ok(res)
    }
}
