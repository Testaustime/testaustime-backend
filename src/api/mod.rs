use std::sync::LazyLock;

use regex::Regex;

pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
#[cfg(feature = "testausid")]
pub mod oauth;
pub mod users;

static VALID_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:word:]]{2,32}$").unwrap());
