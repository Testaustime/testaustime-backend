use std::collections::HashMap;

use itertools::Itertools;
use lettre::Address;
use rand::{distr::Alphanumeric, rng, Rng};

use crate::models::CodingActivity;

pub fn generate_auth_token() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

pub fn generate_friend_code() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

pub fn generate_password_reset_token() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

pub fn group_by_language(iter: impl Iterator<Item = CodingActivity>) -> HashMap<String, i32> {
    iter.map(|d| {
        (
            d.language.unwrap_or_else(|| String::from("none")),
            d.duration,
        )
    })
    .into_grouping_map()
    .sum()
}

pub fn validate_email(email: &str) -> bool {
    email.parse::<Address>().is_ok()
}
