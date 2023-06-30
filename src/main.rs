#![feature(lazy_cell)]

mod api;
mod auth;
mod database;
mod error;
mod models;
mod requests;
mod schema;
mod utils;

use actix::Actor;
use actix_cors::Cors;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    error::{ErrorBadRequest, QueryPayloadError},
    web,
    web::{Data, QueryConfig},
    App, HttpMessage, HttpServer,
};
use auth::{AuthMiddleware, Authentication};
#[cfg(feature = "testausid")]
use awc::Client;
use chrono::NaiveDateTime;
use dashmap::DashMap;
use database::{Database, DatabaseConnectionPool};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use serde_derive::Deserialize;
use testausratelimiter::{RateLimiter, RateLimiterStorage};
use tracing::Span;
use tracing_actix_web::{root_span, RootSpanBuilder, TracingLogger};

#[macro_use]
extern crate actix_web;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate serde_json;

#[derive(Debug, Deserialize)]
pub struct TimeConfig {
    pub ratelimit_by_peer_ip: bool,
    pub max_requests_per_min: usize,
    pub address: String,
    pub database_url: String,
    pub allowed_origin: String,
}

pub struct TestaustimeRootSpanBuilder;

impl RootSpanBuilder for TestaustimeRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        if let Authentication::AuthToken(user) =
            request.extensions().get::<Authentication>().unwrap()
        {
            root_span!(request, user.id, user.username)
        } else {
            root_span!(request)
        }
    }

    fn on_request_end<B>(_span: Span, _outcome: &Result<ServiceResponse<B>, actix_web::Error>) {}
}

type DbConnection = diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub struct RegisterLimiter {
    pub limit_by_peer_ip: bool,
    pub storage: DashMap<String, NaiveDateTime>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config: TimeConfig =
        toml::from_str(&std::fs::read_to_string("settings.toml").expect("Missing settings.toml"))
            .expect("Invalid Toml in settings.toml");

    let manager = ConnectionManager::<PgConnection>::new(config.database_url);

    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create connection pool");

    let database = Data::new(Database {
        backend: Box::new(pool) as Box<dyn DatabaseConnectionPool>,
    });

    let register_limiter = Data::new(RegisterLimiter {
        limit_by_peer_ip: config.ratelimit_by_peer_ip,
        storage: DashMap::new(),
    });

    let ratelimiter = RateLimiterStorage::new(config.max_requests_per_min, 60).start();

    let heartbeat_store = Data::new(api::activity::HeartBeatMemoryStore::new());
    let leaderboard_cache = Data::new(api::leaderboards::LeaderboardCache::new());

    HttpServer::new(move || {
        #[cfg(feature = "testausid")]
        let tracing = TracingLogger::<TestaustimeRootSpanBuilder>::new();
        let client = Client::new();
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "DELETE"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .max_age(3600);
        let query_config = QueryConfig::default().error_handler(|err, _| match err {
            QueryPayloadError::Deserialize(e) => ErrorBadRequest(json!({ "error": e.to_string() })),
            _ => unreachable!(),
        });
        let app = App::new()
            .app_data(Data::clone(&register_limiter))
            .app_data(query_config)
            .wrap(cors)
            .service(api::health)
            .service(api::auth::register)
            .service({
                let scope = web::scope("")
                    .wrap(tracing)
                    .wrap(AuthMiddleware)
                    .wrap(RateLimiter {
                        storage: ratelimiter.clone(),
                        use_peer_addr: config.ratelimit_by_peer_ip,
                        maxrpm: config.max_requests_per_min,
                        reset_interval: 60,
                    })
                    .service({
                        web::scope("/activity")
                            .service(api::activity::update)
                            .service(api::activity::delete)
                            .service(api::activity::flush)
                            .service(api::activity::rename_project)
                    })
                    .service(api::auth::login)
                    .service(api::auth::regenerate)
                    .service(api::auth::changeusername)
                    .service(api::auth::changepassword)
                    .service(api::account::change_settings)
                    .service(api::friends::add_friend)
                    .service(api::friends::get_friends)
                    .service(api::friends::regenerate_friend_code)
                    .service(api::friends::remove)
                    .service(api::users::my_profile)
                    .service(api::users::get_activities)
                    .service(api::users::get_current_activity)
                    .service(api::users::delete_user)
                    .service(api::users::my_leaderboards)
                    .service(api::users::get_activity_summary)
                    .service(api::leaderboards::create_leaderboard)
                    .service(api::leaderboards::get_leaderboard)
                    .service(api::leaderboards::join_leaderboard)
                    .service(api::leaderboards::leave_leaderboard)
                    .service(api::leaderboards::delete_leaderboard)
                    .service(api::leaderboards::promote_member)
                    .service(api::leaderboards::demote_member)
                    .service(api::leaderboards::kick_member)
                    .service(api::leaderboards::regenerate_invite)
                    .service(api::search::search_public_users)
                    .service(api::stats::stats);
                #[cfg(feature = "testausid")]
                {
                    scope.service(api::oauth::callback)
                }
                #[cfg(not(feature = "testausid"))]
                {
                    scope
                }
            })
            .app_data(Data::clone(&leaderboard_cache))
            .app_data(Data::clone(&database))
            .app_data(Data::clone(&heartbeat_store));
        #[cfg(feature = "testausid")]
        {
            app.app_data(Data::new(client))
        }
        #[cfg(not(feature = "testausid"))]
        {
            app
        }
    })
    .bind(config.address)?
    .run()
    .await
}
