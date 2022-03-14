#![feature(let_else)]

mod api;
mod database;
pub mod models;
mod requests;
pub mod schema;
mod user;
mod utils;

use std::collections::HashMap;

use actix::prelude::*;
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
    let heartbeat_store = Data::new(api::HeartBeatMemoryStore::new());
    let maxrpm = std::env::var("MAX_REQUESTS_PER_MINUTE")
        .unwrap_or("10".to_string())
        .parse::<i32>()
        .expect("Invalid request");
    let ratelimiter = RateLimiterStorage::new(maxrpm).start();
    HttpServer::new(move || {
        App::new()
            .wrap(RateLimiter {
                storage: ratelimiter.clone(),
            })
            .wrap(Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .service(api::update)
            .service(api::flush)
            .service(api::get_activities)
            .service(api::register)
            .service(api::login)
            .service(api::regenerate)
            .service(api::add_friend)
            .service(api::get_friends)
            .app_data(Data::clone(&database))
            .app_data(Data::clone(&heartbeat_store))
    })
    .bind(dotenv::var("TESTAUSTIME_ADDRESS").unwrap())?
    .run()
    .await
}
