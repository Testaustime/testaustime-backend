use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    prelude::*,
};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

use crate::models::*;

impl Database {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }

    pub async fn get_user_by_discord_id(&self, discord_user_id: u64) -> Result<Option<RegisteredUser>, anyhow::Error> {
        use crate::schema::RegisteredUsers::dsl::*;
        Ok(RegisteredUsers.filter(discord_id.eq(discord_user_id))
            .first::<RegisteredUser>(&self.pool.get()?).optional()?)
    }

    pub async fn new_user(&self, discord_user_id: u64) -> Result<RegisteredUser, anyhow::Error> {
        let new_user = NewRegisteredUser {
            discord_id: discord_user_id,
            auth_token: crate::utils::generate_token(),
            registration_time: chrono::Local::now().naive_local(),
            user_name: String::from("")
        };
        diesel::insert_into(crate::schema::RegisteredUsers::table)
            .values(&new_user)
            .execute(&self.pool.get()?)?;

        self.get_user_by_discord_id(discord_user_id).await?.ok_or(anyhow::anyhow!("No user found"))
    }
}
