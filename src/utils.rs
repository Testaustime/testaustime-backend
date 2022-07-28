use std::collections::HashMap;

use itertools::Itertools;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::models::CodingActivity;

pub fn generate_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

pub fn generate_friend_code() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
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
