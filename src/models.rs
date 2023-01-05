#![allow(clippy::extra_unused_lifetimes)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct UserId {
    pub id: i32,
}

#[derive(Queryable, Clone, Debug, Serialize, PartialEq, Eq)]
pub struct UserIdentity {
    pub id: i32,
    #[serde(skip_serializing)]
    pub auth_token: String,
    pub friend_code: String,
    pub username: String,
    pub registration_time: chrono::NaiveDateTime,
    pub is_public: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicUser {
    pub id: i32,
    pub username: String,
    pub registration_time: chrono::NaiveDateTime,
}

impl From<UserIdentity> for PublicUser {
    fn from(user_identity: UserIdentity) -> PublicUser {
        PublicUser {
            id: user_identity.id,
            username: user_identity.username,
            registration_time: user_identity.registration_time,
        }
    }
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct TestaustimeUser {
    pub id: i32,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub identity: i32,
}

use crate::schema::testaustime_users;

#[derive(Insertable, Serialize, Clone)]
#[diesel(table_name = testaustime_users)]
pub struct NewTestaustimeUser {
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub identity: i32,
}

// This is here so that vilepis doesn't actually give friends each others' auth tokens
// Fuck off
#[derive(Clone, Debug, Serialize)]
pub struct SelfUser {
    pub id: i32,
    pub auth_token: String,
    pub friend_code: String,
    pub username: String,
    pub registration_time: chrono::NaiveDateTime,
    pub is_public: bool,
}

impl From<UserIdentity> for SelfUser {
    fn from(u: UserIdentity) -> SelfUser {
        SelfUser {
            id: u.id,
            auth_token: u.auth_token,
            friend_code: u.friend_code,
            username: u.username,
            registration_time: u.registration_time,
            is_public: u.is_public,
        }
    }
}

#[cfg(feature = "testausid")]
use crate::schema::testausid_users;

#[cfg(feature = "testausid")]
#[derive(Insertable, Serialize, Clone)]
#[diesel(table_name = testausid_users)]
pub struct NewTestausIdUser {
    pub user_id: String,
    pub service_id: String,
    pub identity: i32,
}

#[cfg(feature = "testausid")]
#[derive(Queryable, Serialize, Clone)]
pub struct TestausIdUser {
    pub id: i32,
    pub user_id: String,
    pub service_id: String,
    pub identity: i32,
}

use crate::schema::user_identities;

#[derive(Insertable, Serialize, Clone)]
#[diesel(table_name = user_identities)]
pub struct NewUserIdentity {
    pub auth_token: String,
    pub username: String,
    pub friend_code: String,
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
#[diesel(table_name = friend_relations)]
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
#[diesel(table_name = coding_activities)]
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
#[diesel(table_name = leaderboards)]
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
#[diesel(table_name = leaderboard_members)]
pub struct NewLeaderboardMember {
    pub leaderboard_id: i32,
    pub user_id: i32,
    pub admin: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct PrivateLeaderboardMember {
    pub username: String,
    pub admin: bool,
    pub time_coded: i32,
}

#[derive(Serialize, Clone, Debug)]
pub struct PrivateLeaderboard {
    pub name: String,
    pub invite: String,
    pub creation_time: chrono::NaiveDateTime,
    pub members: Vec<PrivateLeaderboardMember>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct CodingTimeSteps {
    pub all_time: i32,
    pub past_month: i32,
    pub past_week: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FriendWithTime {
    pub username: String,
    pub coding_time: CodingTimeSteps,
}
