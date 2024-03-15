#![allow(clippy::extra_unused_lifetimes)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct UserId {
    pub id: i32,
}

#[derive(Identifiable, Queryable, Clone, Debug, Serialize, PartialEq, Eq)]
#[diesel(table_name = user_identities)]
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

#[derive(Queryable, Clone, Debug, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(UserIdentity, foreign_key=identity))]
#[diesel(table_name = testaustime_users)]
pub struct TestaustimeUser {
    pub id: i32,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub identity: i32,
}

use crate::{requests::HeartBeat, schema::testaustime_users};

#[derive(Insertable, Serialize, Clone)]
#[diesel(table_name = testaustime_users)]
pub struct NewTestaustimeUser {
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub identity: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Queryable, Serialize, Clone, Associations, Identifiable)]
#[diesel(belongs_to(UserIdentity, foreign_key=identity))]
#[diesel(table_name = testausid_users)]
pub struct TestausIdUser {
    pub id: i32,
    pub user_id: String,
    pub service_id: String,
    pub identity: i32,
}

use crate::schema::user_identities;

#[derive(Insertable, Serialize, Clone, Deserialize)]
#[diesel(table_name = user_identities)]
pub struct NewUserIdentity {
    pub auth_token: String,
    pub username: String,
    pub friend_code: String,
    pub registration_time: chrono::NaiveDateTime,
}

// NOTE: It is impossible to use diesel::assocations here
// https://github.com/diesel-rs/diesel/issues/2142
#[derive(Queryable, Clone, Debug, Identifiable)]
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

#[derive(Queryable, Clone, Debug, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(UserIdentity, foreign_key=user_id))]
#[diesel(table_name = coding_activities)]
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
    pub hidden: bool,
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
    pub hidden: bool,
}

#[derive(Queryable, Clone, Debug, Serialize, Hash, Eq, PartialEq, Identifiable)]
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

#[derive(Queryable, Clone, Debug, Identifiable, Associations)]
#[diesel(belongs_to(Leaderboard))]
#[diesel(belongs_to(UserIdentity, foreign_key=user_id))]
#[diesel(table_name = leaderboard_members)]
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

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct PrivateLeaderboardMember {
    pub id: i32,
    pub username: String,
    pub admin: bool,
    pub time_coded: i32,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
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

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Clone)]
pub struct CurrentActivity {
    pub started: chrono::NaiveDateTime,
    pub duration: i64,
    pub heartbeat: HeartBeat,
}

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct FriendWithTime {
    pub user: UserIdentity,
    pub coding_time: CodingTimeSteps,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FriendWithTimeAndStatus {
    pub username: String,
    pub coding_time: CodingTimeSteps,
    pub status: Option<CurrentActivity>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct SecuredAccessTokenResponse {
    pub token: String,
}
