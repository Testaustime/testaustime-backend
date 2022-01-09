use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

impl Database {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }
}
