use std::sync::LazyLock;

use actix_web::{HttpResponse, Responder};
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

static VALID_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:word:]]{2,32}$").unwrap());

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok()
}
