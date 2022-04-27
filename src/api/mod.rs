use std::lazy::SyncLazy;

use regex::Regex;

pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
pub mod users;

static VALID_NAME_REGEX: SyncLazy<Regex> =
    SyncLazy::new(|| Regex::new("^[[:word:]]{2,32}$").unwrap());
