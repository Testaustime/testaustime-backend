use chrono::{serde::ts_seconds_option, DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Clone)]
pub struct HeartBeat {
    #[serde(deserialize_with = "project_deserialize")]
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
}

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

#[derive(Deserialize, Debug)]
pub struct DataRequest {
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    pub to: Option<DateTime<Utc>>,
    pub min_duration: Option<i32>,
    pub editor_name: Option<String>,
    pub language: Option<String>,
    pub hostname: Option<String>,
    pub project_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UsernameChangeRequest {
    pub new: String,
}

#[derive(Deserialize)]
pub struct PasswordChangeRequest {
    pub old: String,
    pub new: String,
}

#[derive(Deserialize, Debug)]
pub struct FriendRequest {
    pub code: String,
}
