mod api;
mod auth;
mod database;
mod error;
mod models;
mod ratelimiter;
mod schema;
mod state;
mod utils;

#[cfg(test)]
mod tests;

use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

use api::activity::HeartBeatMemoryStore;
use auth::{AuthMiddleware, Authentication};
use axum::{
    body::Body,
    routing::{delete, get, post},
    Router,
};
use chrono::NaiveDateTime;
use dashmap::DashMap;
use database::Database;
use governor::{Quota, RateLimiter};
use lettre::{transport::smtp::authentication::Credentials, AsyncSmtpTransport, Tokio1Executor};
use models::UserIdentity;
use ratelimiter::TestaustimeRateLimiter;
use serde_derive::Deserialize;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate serde_json;

#[derive(Debug, Deserialize)]
pub struct TestaustimeConfig {
    pub bypass_token: String,
    pub ratelimit_by_peer_ip: bool,
    pub max_requests_per_min: u32,
    pub max_registers_per_hour: u32,
    pub address: String,
    pub database_url: String,
    pub allowed_origin: String,
    pub mail_server: String,
    pub mail_user: String,
    pub mail_password: String,
}

pub struct RegisterLimiter {
    pub limit_by_peer_ip: bool,
    pub storage: DashMap<String, NaiveDateTime>,
}

pub struct PasswordReset {
    pub user: UserIdentity,
    pub expires: NaiveDateTime,
}

pub struct PasswordResetState {
    pub storage: DashMap<String, PasswordReset>,
}

#[derive(Clone)]
pub struct TestaustimeState {
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    database: Arc<Database>,
    heartbeat_store: Arc<HeartBeatMemoryStore>,
    register_limiter: Arc<RegisterLimiter>,
    password_reset_state: Arc<PasswordResetState>,
}

fn create_router(config: &TestaustimeConfig) -> Router {
    let database = Arc::new(Database::new(config.database_url.clone()));

    let register_limiter = Arc::new(RegisterLimiter {
        limit_by_peer_ip: config.ratelimit_by_peer_ip,
        storage: DashMap::new(),
    });

    let heartbeat_store = Arc::new(HeartBeatMemoryStore::new());

    let password_reset_state = Arc::new(PasswordResetState {
        storage: DashMap::default(),
    });

    let creds = Credentials::new(config.mail_user.to_owned(), config.mail_password.to_owned());

    let smtp: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.mail_server)
            .unwrap_or_else(|_| panic!("failed to connect to {}", &config.mail_server))
            .credentials(creds)
            .build();

    debug!(
        "Conntected to mail server on {} as {}",
        config.mail_server, config.mail_user
    );

    let state = TestaustimeState {
        smtp,
        heartbeat_store,
        database,
        register_limiter,
        password_reset_state,
    };

    let auth = AuthMiddleware {
        state: state.clone(),
    };

    let ratelimiter = Arc::new(
        RateLimiter::keyed(Quota::per_minute(
            NonZeroU32::new(config.max_requests_per_min).unwrap(),
        ))
        .with_middleware(),
    );

    let register_ratelimiter = Arc::new(
        RateLimiter::keyed(Quota::per_hour(
            NonZeroU32::new(config.max_registers_per_hour).unwrap(),
        ))
        .with_middleware(),
    );

    Router::new()
        .route("/health", get(api::health))
        .merge(
            Router::new()
                .route("/auth/register", post(api::auth::register))
                .layer(TestaustimeRateLimiter {
                    limiter: register_ratelimiter,
                    use_peer_addr: config.ratelimit_by_peer_ip,
                    bypass_token: config.bypass_token.clone(),
                }),
        )
        .merge({
            let router = Router::new()
                .nest("/activity", {
                    Router::new()
                        .route("/update", post(api::activity::update))
                        .route("/delete", delete(api::activity::delete))
                        .route("/flush", post(api::activity::flush))
                        .route("/rename", post(api::activity::rename_project))
                        .route("/hide", post(api::activity::hide_project))
                })
                .route("/auth/login", post(api::auth::login))
                .route("/auth/change-username", post(api::auth::change_username))
                .route("/auth/change-email", post(api::auth::change_email))
                .route("/auth/change-password", post(api::auth::change_password))
                .route(
                    "/auth/reset-password",
                    post(api::auth::request_password_reset),
                )
                .route(
                    "/auth/complete-password-reset",
                    post(api::auth::reset_password),
                )
                .route("/friends/add", post(api::friends::add_friend))
                .route("/account/settings", post(api::account::change_settings))
                .route("/friends/list", get(api::friends::get_friends))
                .route(
                    "/friends/regenerate",
                    get(api::friends::regenerate_friend_code),
                )
                .route("/friends/remove", delete(api::friends::remove))
                .route("/users/@me", get(api::users::my_profile))
                .route("/users/@me/delete", delete(api::users::delete_user))
                .route("/users/@me/leaderboards", get(api::users::my_leaderboards))
                .route(
                    "/users/{username}/activity/data",
                    get(api::users::get_activities),
                )
                .route(
                    "/users/{username}/activity/current",
                    get(api::users::get_current_activity),
                )
                .route(
                    "/users/{username}/activity/summary",
                    get(api::users::get_activity_summary),
                )
                .route(
                    "/leaderboards/create",
                    post(api::leaderboards::create_leaderboard),
                )
                .route(
                    "/leaderboards/{name}",
                    get(api::leaderboards::get_leaderboard),
                )
                .route(
                    "/leaderboards/join",
                    post(api::leaderboards::join_leaderboard),
                )
                .route(
                    "/leaderboards/{name}/leave",
                    post(api::leaderboards::leave_leaderboard),
                )
                .route(
                    "/leaderboards/{name}",
                    delete(api::leaderboards::delete_leaderboard),
                )
                .route(
                    "/leaderboards/{name}/promote",
                    post(api::leaderboards::promote_member),
                )
                .route(
                    "/leaderboards/{name}/demote",
                    post(api::leaderboards::demote_member),
                )
                .route(
                    "/leaderboards/{name}/kick",
                    post(api::leaderboards::kick_member),
                )
                .route(
                    "/leaderboards/{name}/regenerate",
                    post(api::leaderboards::regenerate_invite),
                )
                .route("/search/users", get(api::search::search_public_users))
                .route("/stats", get(api::stats::stats));

            #[cfg(feature = "testausid")]
            let router = router.route("/auth/callback", get(api::oauth::callback));

            router
                .layer(
                    ServiceBuilder::new()
                        .layer(TestaustimeRateLimiter {
                            limiter: ratelimiter,
                            use_peer_addr: config.ratelimit_by_peer_ip,
                            bypass_token: config.bypass_token.clone(),
                        })
                        .layer(auth),
                )
                .layer(TraceLayer::new_for_http().make_span_with(
                    |request: &http::Request<Body>| {
                        tracing::debug_span!(
                            "request",
                            method = %request.method(),
                            uri = request.uri().path(),
                            user = request
                                .extensions()
                                .get::<Authentication>()
                                .and_then(|auth| auth.user().map(|u| &u.username)))
                    },
                ))
        })
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_env("TESTAUSTIME_LOG").unwrap_or_else(|_| {
                "testaustime=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config: TestaustimeConfig =
        toml::from_str(&std::fs::read_to_string("settings.toml").expect("Missing settings.toml"))
            .expect("Invalid Toml in settings.toml");

    let router = create_router(&config);

    let listener = tokio::net::TcpListener::bind(&config.address)
        .await
        .unwrap();

    info!("Staring server on {}", config.address);

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
