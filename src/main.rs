#![feature(let_else, once_cell)]

mod api;
mod database;
mod error;
pub mod models;
mod requests;
pub mod schema;
mod user;
mod utils;

use std::collections::HashMap;

use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use testausratelimiter::*;

#[macro_use]
extern crate actix_web;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    RateLimiterStorage {
        clients: HashMap::new(),
        maxrpm: 10,
    }
    .start();

    let database = Data::new(database::Database::new());
    let heartbeat_store = Data::new(api::activity::HeartBeatMemoryStore::new());
    let maxrpm = std::env::var("MAX_REQUESTS_PER_MINUTE")
        .unwrap_or("10".to_string())
        .parse::<i32>()
        .expect("Invalid request");
    let ratelimiter = RateLimiterStorage::new(maxrpm).start();
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);
        App::new()
            .wrap(cors)
            .wrap(RateLimiter {
                storage: ratelimiter.clone(),
            })
            .wrap(Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .service(api::activity::update)
            .service(api::activity::flush)
            .service(api::activity::get_activities)
            .service(api::auth::register)
            .service(api::auth::login)
            .service(api::auth::regenerate)
            .service(api::friends::add_friend)
            .service(api::friends::get_friends)
            .service(api::friends::regenerate_friend_code)
            .service(api::users::my_profile)
            .app_data(Data::clone(&database))
            .app_data(Data::clone(&heartbeat_store))
    })
    .bind(dotenv::var("TESTAUSTIME_ADDRESS").unwrap())?
    .run()
    .await
}
