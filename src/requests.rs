use chrono::{serde::ts_seconds_option, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Clone)]
pub struct HeartBeat {
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
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

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct FriendRequest {
    pub code: String,
}
