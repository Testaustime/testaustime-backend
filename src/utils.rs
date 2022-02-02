use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub fn generate_token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
