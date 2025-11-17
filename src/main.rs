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
use axum::{body::Body, Router};
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
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

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

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Bearer")
                        .build(),
                ),
            )
        }
    }
}

fn create_router_with_openapi(config: &TestaustimeConfig) -> (Router, utoipa::openapi::OpenApi) {
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

    OpenApiRouter::new()
        .routes(routes!(api::health))
        .merge(
            OpenApiRouter::new()
                .routes(routes!(api::auth::register))
                .layer(TestaustimeRateLimiter {
                    limiter: register_ratelimiter,
                    use_peer_addr: config.ratelimit_by_peer_ip,
                    bypass_token: config.bypass_token.clone(),
                }),
        )
        .merge({
            let router = OpenApiRouter::with_openapi(ApiDoc::openapi())
                .routes(routes!(api::activity::update))
                .routes(routes!(api::activity::delete))
                .routes(routes!(api::activity::flush))
                .routes(routes!(api::activity::rename_project))
                .routes(routes!(api::activity::hide_project))
                .routes(routes!(api::auth::login))
                .routes(routes!(api::auth::change_username))
                .routes(routes!(api::auth::change_email))
                .routes(routes!(api::auth::change_password))
                .routes(routes!(api::auth::request_password_reset))
                .routes(routes!(api::auth::reset_password))
                .routes(routes!(api::account::change_settings))
                .routes(routes!(api::friends::add_friend))
                .routes(routes!(api::friends::get_friends))
                .routes(routes!(api::friends::regenerate_friend_code))
                .routes(routes!(api::friends::remove))
                .routes(routes!(api::users::my_profile))
                .routes(routes!(api::users::delete_user))
                .routes(routes!(api::users::my_leaderboards))
                .routes(routes!(api::users::get_activities))
                .routes(routes!(api::users::get_current_activity))
                .routes(routes!(api::users::get_activity_summary))
                .routes(routes!(api::leaderboards::create_leaderboard))
                .routes(routes!(api::leaderboards::get_leaderboard))
                .routes(routes!(api::leaderboards::join_leaderboard))
                .routes(routes!(api::leaderboards::leave_leaderboard))
                .routes(routes!(api::leaderboards::delete_leaderboard))
                .routes(routes!(api::leaderboards::promote_member))
                .routes(routes!(api::leaderboards::demote_member))
                .routes(routes!(api::leaderboards::kick_member))
                .routes(routes!(api::leaderboards::regenerate_invite))
                .routes(routes!(api::search::search_public_users))
                .routes(routes!(api::stats::stats));

            #[cfg(feature = "testausid")]
            let router = router.routes(routes!(api::oauth::callback));

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
        .split_for_parts()
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

    let (router, openapi) = create_router_with_openapi(&config);
    let router =
        router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()));

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
