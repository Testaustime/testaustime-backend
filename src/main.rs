#![feature(let_else)]

mod api;
mod database;
pub mod models;
pub mod schema;
mod user;
mod utils;

use actix_web::{middleware::Logger, web::Data, App, HttpServer};

#[macro_use]
extern crate actix_web;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let database = Data::new(database::Database::new());
    let heartbeat_store = Data::new(api::HeartBeatMemoryStore::new());
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(api::update)
            .service(api::get_activities)
            .app_data(Data::clone(&database))
            .app_data(Data::clone(&heartbeat_store))
    })
    .bind(dotenv::var("TESTAUSTIME_ADDRESS").unwrap())?
    .run()
    .await
}
