use std::sync::LazyLock;

use http::StatusCode;
use regex::Regex;

pub mod account;
pub mod activity;
pub mod auth;
pub mod friends;
pub mod leaderboards;
#[cfg(feature = "testausid")]
pub mod oauth;
pub mod search;
pub mod stats;
pub mod users;

pub static VALID_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:word:]]{2,32}$").unwrap());

/// Check API health status.
///
/// Returns OK if the API is running and healthy.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = OK, description = "API is healthy")
    )
)]
pub async fn health() -> StatusCode {
    StatusCode::OK
}
