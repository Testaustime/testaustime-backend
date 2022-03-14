use serde::Serialize;

#[derive(Queryable, Clone, Debug)]
pub struct RegisteredUser {
    pub id: i32,
    pub auth_token: String,
    pub user_name: String,
    pub friend_code: Option<String>,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub registration_time: chrono::NaiveDateTime,
}

use crate::schema::RegisteredUsers;

#[derive(Insertable)]
#[table_name = "RegisteredUsers"]
pub struct NewRegisteredUser<'a> {
    pub auth_token: String,
    pub user_name: String,
    pub friend_code: Option<String>,
    pub password: &'a [u8],
    pub salt: &'a [u8],
    pub registration_time: chrono::NaiveDateTime,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct CodingActivity {
    #[serde(skip_serializing)]
    pub id: i32,
    #[serde(skip_serializing)]
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
