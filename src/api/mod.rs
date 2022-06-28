use std::sync::LazyLock;

use regex::Regex;

pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
pub mod users;

static VALID_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:word:]]{2,32}$").unwrap());
