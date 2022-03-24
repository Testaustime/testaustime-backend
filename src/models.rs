use serde::Serialize;

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct RegisteredUser {
    pub id: i32,
    #[serde(skip_serializing)]
    pub auth_token: String,
    pub friend_code: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub registration_time: chrono::NaiveDateTime,
}

use crate::schema::registered_users;

#[derive(Insertable, Serialize, Clone)]
#[table_name = "registered_users"]
pub struct NewRegisteredUser {
    pub auth_token: String,
    pub username: String,
    pub friend_code: String,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub registration_time: chrono::NaiveDateTime,
}

#[derive(Queryable, Clone, Debug)]
pub struct FriendRelation {
    pub id: i32,
    pub lesser_id: i32,
    pub greater_id: i32,
}

use crate::schema::friend_relations;

#[derive(Insertable)]
#[table_name = "friend_relations"]
pub struct NewFriendRelation {
    pub lesser_id: i32,
    pub greater_id: i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct CodingActivity {
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

use crate::schema::coding_activities;

#[derive(Insertable)]
#[table_name = "coding_activities"]
pub struct NewCodingActivity {
    pub user_id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub duration: i32,
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}
