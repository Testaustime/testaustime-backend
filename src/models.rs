#![allow(clippy::extra_unused_lifetimes)]
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct UserId {
    pub id: i32,
}

/// A user's identity and profile information.
#[derive(Identifiable, Queryable, Clone, Debug, Serialize, PartialEq, Eq, ToSchema)]
#[diesel(table_name = user_identities)]
pub struct UserIdentity {
    /// Unique user identifier.
    pub id: i32,
    #[serde(skip_serializing)]
    pub auth_token: String,
    /// Code for others to add this user as a friend (starts with ttfc_).
    pub friend_code: String,
    /// The user's display name.
    pub username: String,
    /// When the user registered.
    pub registration_time: chrono::NaiveDateTime,
    /// Whether the user's profile is publicly visible.
    pub is_public: bool,
    /// The user's email address (optional).
    pub email: Option<String>,
}

/// Public user information visible to anyone.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PublicUser {
    /// Unique user identifier.
    pub id: i32,
    /// The user's display name.
    pub username: String,
    /// When the user registered.
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
    pub identity: i32,
    #[serde(skip_serializing)]
    pub password: String,
}

use crate::schema::testaustime_users;

#[derive(Insertable, Serialize, Clone)]
#[diesel(table_name = testaustime_users)]
pub struct NewTestaustimeUser {
    pub identity: i32,
    #[serde(skip_serializing)]
    pub password: String,
}

/// Full user profile returned to the authenticated user.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SelfUser {
    /// Unique user identifier.
    pub id: i32,
    /// Authentication token for API requests.
    pub auth_token: String,
    /// Code for others to add this user as a friend (starts with ttfc_).
    pub friend_code: String,
    /// The user's display name.
    pub username: String,
    /// When the user registered.
    pub registration_time: chrono::NaiveDateTime,
    /// Whether the user's profile is publicly visible.
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

/// A newly created user identity.
#[derive(Insertable, Serialize, ToSchema, Clone, Deserialize)]
#[diesel(table_name = user_identities)]
pub struct NewUserIdentity {
    /// Authentication token for API requests.
    pub auth_token: String,
    /// The user's display name.
    pub username: String,
    /// Code for others to add this user as a friend (starts with ttfc_).
    pub friend_code: String,
    /// When the user registered.
    pub registration_time: chrono::NaiveDateTime,
    /// The user's email address (optional).
    pub email: Option<String>,
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

/// A recorded coding activity session.
#[derive(Queryable, Clone, Debug, Serialize, Identifiable, Associations, ToSchema)]
#[diesel(belongs_to(UserIdentity, foreign_key=user_id))]
#[diesel(table_name = coding_activities)]
pub struct CodingActivity {
    /// Unique activity identifier.
    pub id: i32,
    #[serde(skip_serializing)]
    pub user_id: i32,
    /// When the coding session started.
    pub start_time: chrono::NaiveDateTime,
    /// Duration of the session in seconds.
    pub duration: i32,
    /// Name of the project being worked on.
    pub project_name: Option<String>,
    /// Programming language used.
    pub language: Option<String>,
    /// Name of the editor/IDE.
    pub editor_name: Option<String>,
    /// Hostname of the machine.
    pub hostname: Option<String>,
    /// Whether this activity is hidden from friends and public view.
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

/// A member of a leaderboard with their stats.
#[derive(Serialize, Clone, Debug, Deserialize, ToSchema)]
pub struct PrivateLeaderboardMember {
    /// Unique user identifier.
    pub id: i32,
    /// The member's display name.
    pub username: String,
    /// Whether the member is an admin of this leaderboard.
    pub admin: bool,
    /// Total coding time in seconds (last 7 days).
    pub time_coded: i32,
}

/// Full leaderboard details including all members.
#[derive(Serialize, Clone, Debug, Deserialize, ToSchema)]
pub struct PrivateLeaderboard {
    /// Name of the leaderboard.
    pub name: String,
    /// Invite code for others to join (starts with ttlic_).
    pub invite: String,
    /// When the leaderboard was created.
    pub creation_time: chrono::NaiveDateTime,
    /// List of all members with their stats.
    pub members: Vec<PrivateLeaderboardMember>,
}

/// Coding time statistics over different time periods.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash, ToSchema)]
pub struct CodingTimeSteps {
    /// Total coding time in seconds (all time).
    pub all_time: i32,
    /// Coding time in seconds (last 30 days).
    pub past_month: i32,
    /// Coding time in seconds (last 7 days).
    pub past_week: i32,
}

/// A user's current coding activity (if active).
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Clone, ToSchema)]
pub struct CurrentActivity {
    /// When the current session started.
    pub started: chrono::NaiveDateTime,
    /// Duration of the current session in seconds.
    pub duration: i64,
    /// Details of what the user is working on.
    pub heartbeat: HeartBeat,
}

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct FriendWithTime {
    pub user: UserIdentity,
    pub coding_time: CodingTimeSteps,
}

/// Friend information with coding stats and current status.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash, ToSchema)]
pub struct FriendWithTimeAndStatus {
    /// The friend's display name.
    pub username: String,
    /// The friend's coding time statistics.
    pub coding_time: CodingTimeSteps,
    /// The friend's current activity (if currently coding).
    pub status: Option<CurrentActivity>,
}

/// A heartbeat sent by editor extensions to track coding activity.
#[derive(Deserialize, Serialize, ToSchema, Debug, Hash, Eq, PartialEq, Clone)]
pub struct HeartBeat {
    /// Name of the project being worked on.
    #[serde(deserialize_with = "project_deserialize")]
    pub project_name: Option<String>,
    /// Programming language of the current file.
    pub language: Option<String>,
    /// Name of the editor/IDE.
    pub editor_name: Option<String>,
    /// Hostname of the machine.
    pub hostname: Option<String>,
    /// Whether this activity should be hidden from friends and public view.
    pub hidden: Option<bool>,
}

// Wtf is this
fn project_deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let project = Option::<String>::deserialize(deserializer)?;
    Ok(project.map(|p| {
        if p.starts_with("tmp.") {
            String::from("tmp")
        } else {
            p
        }
    }))
}
