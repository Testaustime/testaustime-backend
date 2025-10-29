use std::sync::Arc;

use axum::extract::FromRef;
use lettre::{AsyncSmtpTransport, Tokio1Executor};

use crate::{
    api::activity::HeartBeatMemoryStore,
    PasswordResetState, RegisterLimiter, TestaustimeState,
};

impl FromRef<TestaustimeState> for Arc<HeartBeatMemoryStore> {
    fn from_ref(input: &TestaustimeState) -> Self {
        Arc::clone(&input.heartbeat_store)
    }
}

impl FromRef<TestaustimeState> for Arc<RegisterLimiter> {
    fn from_ref(input: &TestaustimeState) -> Self {
        Arc::clone(&input.register_limiter)
    }
}

impl FromRef<TestaustimeState> for Arc<PasswordResetState> {
    fn from_ref(input: &TestaustimeState) -> Self {
        Arc::clone(&input.password_reset_state)
    }
}

impl FromRef<TestaustimeState> for AsyncSmtpTransport<Tokio1Executor> {
    fn from_ref(input: &TestaustimeState) -> Self {
        input.smtp.clone()
    }
}
