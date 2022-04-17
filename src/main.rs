#![feature(let_else, once_cell)]

mod api;
mod database;
mod error;
pub mod models;
mod requests;
pub mod schema;
mod user;
mod utils;

use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, web::Data, App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use serde_derive::Deserialize;
use testausratelimiter::*;

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
    pub ratelimit_by_peer_ip: Option<bool>,
    pub max_requests_per_min: Option<usize>,
    pub max_heartbeats_per_min: Option<usize>,
    pub address: String,
    pub database_url: String,
    pub allowed_origin: String,
}

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config: TimeConfig =
        toml::from_str(&std::fs::read_to_string("settings.toml").expect("Missing settings.toml"))
            .expect("Invalid Toml in settings.toml");

    let manager = ConnectionManager::<PgConnection>::new(config.database_url);
    let pool = Data::new(
        Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool"),
    );

    let heartbeat_store = Data::new(api::activity::HeartBeatMemoryStore::new());
    let ratelimiter = RateLimiterStorage::new(config.max_requests_per_min.unwrap_or(8)).start();
    let heartbeat_ratelimiter =
        RateLimiterStorage::new(config.max_heartbeats_per_min.unwrap_or(30)).start();
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&config.allowed_origin)
            .allowed_origin("https://testaustime.fi")
            .allowed_methods(vec!["GET", "POST", "DELETE"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .max_age(3600);
        App::new()
            .wrap(cors)
            .wrap(Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %Dms"#,
            ))
            .service(
                web::scope("/activity")
                    .wrap(RateLimiter {
                        storage: heartbeat_ratelimiter.clone(),
                        use_peer_addr: config.ratelimit_by_peer_ip.unwrap_or(true),
                        maxrpm: config.max_requests_per_min.unwrap_or(10),
                    })
                    .service(api::activity::update)
                    .service(api::activity::flush),
            )
            .service(
                web::scope("")
                    .wrap(RateLimiter {
                        storage: ratelimiter.clone(),
                        use_peer_addr: config.ratelimit_by_peer_ip.unwrap_or(true),
                        maxrpm: config.max_heartbeats_per_min.unwrap_or(30),
                    })
                    .service(api::activity::delete)
                    .service(api::auth::register)
                    .service(api::auth::login)
                    .service(api::auth::regenerate)
                    .service(api::auth::changeusername)
                    .service(api::auth::changepassword)
                    .service(api::friends::add_friend)
                    .service(api::friends::get_friends)
                    .service(api::friends::regenerate_friend_code)
                    .service(api::friends::remove)
                    .service(api::users::my_profile)
                    .service(api::users::get_activities)
                    .service(api::users::delete_user)
                    .service(api::users::my_leaderboards)
                    .service(api::leaderboards::create_leaderboard)
                    .service(api::leaderboards::get_leaderboard)
                    .service(api::leaderboards::join_leaderboard)
                    .service(api::leaderboards::leave_leaderboard)
                    .service(api::leaderboards::delete_leaderboard)
                    .service(api::leaderboards::promote_member)
                    .service(api::leaderboards::demote_member)
                    .service(api::leaderboards::kick_member)
                    .service(api::leaderboards::regenerate_invite),
            )
            .app_data(Data::clone(&pool))
            .app_data(Data::clone(&heartbeat_store))
    })
    .bind(config.address)?
    .run()
    .await
}
