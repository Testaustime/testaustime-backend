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

// This is here so that vilepis doesn't actually give friends eachothers auth tokens
#[derive(Clone, Debug, Serialize)]
pub struct SelfUser {
    pub id: i32,
    pub auth_token: String,
    pub friend_code: String,
    pub username: String,
    pub registration_time: chrono::NaiveDateTime,
}

impl From<RegisteredUser> for SelfUser {
    fn from(u: RegisteredUser) -> SelfUser {
        SelfUser {
            id: u.id,
            auth_token: u.auth_token,
            friend_code: u.friend_code,
            username: u.username,
            registration_time: u.registration_time,
        }
    }
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

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct Leaderboard {
    pub id: i32,
    pub name: String,
    pub invite_code: String,
    pub creation_time: chrono::NaiveDateTime,
}

use crate::schema::leaderboards;

#[derive(Insertable)]
#[table_name = "leaderboards"]
pub struct NewLeaderboard {
    pub name: String,
    pub invite_code: String,
    pub creation_time: chrono::NaiveDateTime,
}

#[derive(Queryable, Clone, Debug)]
pub struct LeaderboardMember {
    pub id: i32,
    pub leaderboard_id: i32,
    pub user_id: i32,
    pub admin: bool,
}

use crate::schema::leaderboard_members;

#[derive(Insertable)]
#[table_name = "leaderboard_members"]
pub struct NewLeaderboardMember {
    pub leaderboard_id: i32,
    pub user_id: i32,
    pub admin: bool,
}
