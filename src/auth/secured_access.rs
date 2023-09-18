use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::utils::generate_token;

#[derive(Clone, Copy, Debug)]
pub enum SecuredAccessError {
    InvalidToken,
    ExpiredToken,
}

#[derive(Clone)]
pub struct SecuredAccessTokenInstance {
    pub user_id: i32,
    pub expires: Instant,
}

pub struct SecuredAccessTokenStorage {
    inner: DashMap<String, SecuredAccessTokenInstance>,
}

impl SecuredAccessTokenStorage {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub fn get(&self, token: &str) -> Result<SecuredAccessTokenInstance, SecuredAccessError> {
        let instance = self
            .inner
            .get(token)
            .ok_or(SecuredAccessError::InvalidToken)?
            .clone();

        if instance.expires < Instant::now() {
            self.inner.remove(token);
            Err(SecuredAccessError::ExpiredToken)
        } else {
            Ok(instance)
        }
    }

    pub fn create_token(&self, user_id: i32) -> String {
        let token = generate_token();

        self.inner.insert(
            token.clone(),
            SecuredAccessTokenInstance {
                user_id,
                expires: Instant::now() + Duration::from_secs(60 * 60),
            },
        );

        let now = Instant::now();

        self.inner.retain(|_, v| v.expires > now);

        token
    }
}
