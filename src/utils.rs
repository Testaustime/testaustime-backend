use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub fn generate_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
