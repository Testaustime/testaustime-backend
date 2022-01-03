#[derive(Queryable, Clone, Debug)]
pub struct RegisteredUser {
    pub id: i32,
    pub auth_token: String,
    pub discord_id: i64,
    pub registration_time: chrono::NaiveDateTime,
}

use crate::schema::RegisteredUsers;

#[derive(Insertable)]
#[table_name = "RegisteredUsers"]
pub struct NewRegisteredUser {
    pub auth_token: String,
    pub discord_id: i64,
    pub registration_time: chrono::NaiveDateTime,
}

#[derive(Queryable, Clone, Debug)]
pub struct CodingActivity {
    pub id: i32,
    pub user_id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub duration: i32,
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}

use crate::schema::CodingActivities;

#[derive(Insertable)]
#[table_name = "CodingActivities"]
pub struct NewCodingActivity {
    pub user_id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub duration: i32,
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}
